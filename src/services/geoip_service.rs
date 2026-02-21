use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::TtlCache;

const IPWHO_BASE_URL: &str = "https://ipwho.is";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
const SUCCESS_CACHE_TTL: Duration = Duration::from_secs(60 * 60 * 24);
const FAILED_CACHE_TTL: Duration = Duration::from_secs(60 * 10);
const MIN_REQUEST_INTERVAL: Duration = Duration::from_secs(1);
const WINDOW_DURATION: Duration = Duration::from_secs(60);
const MAX_REQUESTS_PER_WINDOW: usize = 58;
const PROVIDER_BLOCK_FALLBACK: Duration = Duration::from_secs(60 * 5);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoIpInfo {
    pub city: String,
    pub region: String,
    pub country: String,
    pub country_code: String,
    pub timezone: String,
    pub org: String,
    pub flag_url: String,
}

#[derive(Debug, Deserialize)]
struct GeoIpLookupResponse {
    success: Option<bool>,
    country: Option<String>,
    country_code: Option<String>,
    region: Option<String>,
    city: Option<String>,
    timezone: Option<GeoIpTimezone>,
    connection: Option<GeoIpConnection>,
    flag: Option<GeoIpFlag>,
}

#[derive(Debug, Deserialize)]
struct GeoIpTimezone {
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeoIpConnection {
    org: Option<String>,
    isp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeoIpFlag {
    img: Option<String>,
}

#[derive(Clone)]
pub struct GeoIpService {
    enabled: bool,
    provider_base_url: String,
    client: reqwest::Client,
    cache: Arc<TtlCache<String, Option<GeoIpInfo>>>,
    state: Arc<Mutex<GeoIpState>>,
}

#[derive(Default)]
struct GeoIpState {
    recent_requests: VecDeque<Instant>,
    last_request_at: Option<Instant>,
    blocked_until: Option<Instant>,
}

impl GeoIpService {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            provider_base_url: IPWHO_BASE_URL.to_string(),
            client: reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()
                .expect("geoip http client must be constructable"),
            cache: Arc::new(TtlCache::new()),
            state: Arc::new(Mutex::new(GeoIpState::default())),
        }
    }

    pub async fn lookup_ip(&self, raw_ip: &str) -> Option<GeoIpInfo> {
        if !self.enabled {
            log::debug!("GeoIpService: disabled, skipping lookup");
            return None;
        }

        let ip = normalize_ip_for_lookup(raw_ip)?;
        if !is_public_ip(&ip) {
            log::debug!("GeoIpService: ip={} is not public, skipping lookup", ip);
            return None;
        }

        if let Some(cached) = self.cache.get(&ip).await {
            log::debug!(
                "GeoIpService: cache hit for ip={} has_value={}",
                ip,
                cached.is_some()
            );
            return cached;
        }
        log::debug!("GeoIpService: cache miss for ip={}", ip);

        if !self.acquire_provider_slot().await {
            log::debug!("GeoIpService: provider slot denied for ip={}", ip);
            return None;
        }

        match self.fetch_provider_geoip(&ip).await {
            GeoIpFetchResult::Success(value) => {
                self.cache
                    .set(ip, Some(value.clone()), SUCCESS_CACHE_TTL)
                    .await;
                log::debug!(
                    "GeoIpService: cached successful response for {}s",
                    SUCCESS_CACHE_TTL.as_secs()
                );
                Some(value)
            }
            GeoIpFetchResult::RateLimited(retry_after) => {
                self.apply_provider_block(retry_after).await;
                self.cache.set(ip, None, FAILED_CACHE_TTL).await;
                log::debug!(
                    "GeoIpService: provider rate-limited, cached empty response for {}s",
                    FAILED_CACHE_TTL.as_secs()
                );
                None
            }
            GeoIpFetchResult::Unavailable => {
                self.cache.set(ip, None, FAILED_CACHE_TTL).await;
                log::debug!(
                    "GeoIpService: provider unavailable, cached empty response for {}s",
                    FAILED_CACHE_TTL.as_secs()
                );
                None
            }
        }
    }

    async fn acquire_provider_slot(&self) -> bool {
        let now = Instant::now();
        let mut state = self.state.lock().await;

        if state.blocked_until.is_some_and(|until| until > now) {
            log::debug!("GeoIpService: provider is currently blocked by retry-after window");
            return false;
        }
        state.blocked_until = None;

        while state
            .recent_requests
            .front()
            .is_some_and(|stamp| now.duration_since(*stamp) >= WINDOW_DURATION)
        {
            state.recent_requests.pop_front();
        }

        if state.recent_requests.len() >= MAX_REQUESTS_PER_WINDOW {
            log::debug!(
                "GeoIpService: request window limit reached ({})",
                MAX_REQUESTS_PER_WINDOW
            );
            return false;
        }

        if state
            .last_request_at
            .is_some_and(|stamp| now.duration_since(stamp) < MIN_REQUEST_INTERVAL)
        {
            log::debug!(
                "GeoIpService: min interval not elapsed ({}ms)",
                MIN_REQUEST_INTERVAL.as_millis()
            );
            return false;
        }

        state.last_request_at = Some(now);
        state.recent_requests.push_back(now);
        true
    }

    async fn apply_provider_block(&self, retry_after: Option<Duration>) {
        let block_for = retry_after.unwrap_or(PROVIDER_BLOCK_FALLBACK);
        log::warn!(
            "GeoIpService: applying provider block for {}s",
            block_for.as_secs()
        );
        let mut state = self.state.lock().await;
        state.blocked_until = Some(Instant::now() + block_for);
    }

    async fn fetch_provider_geoip(&self, ip: &str) -> GeoIpFetchResult {
        let base_url = self.provider_base_url.trim_end_matches('/');
        let response = match self.client.get(format!("{base_url}/{ip}")).send().await {
            Ok(response) => response,
            Err(error) => {
                log::warn!("GeoIP provider request failed for {}: {}", ip, error);
                return GeoIpFetchResult::Unavailable;
            }
        };

        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .map(Duration::from_secs);
            return GeoIpFetchResult::RateLimited(retry_after);
        }

        if !response.status().is_success() {
            log::warn!(
                "GeoIP provider returned status {} for {}",
                response.status(),
                ip
            );
            return GeoIpFetchResult::Unavailable;
        }

        let payload = match response.json::<GeoIpLookupResponse>().await {
            Ok(payload) => payload,
            Err(error) => {
                log::warn!("GeoIP provider JSON parse failed for {}: {}", ip, error);
                return GeoIpFetchResult::Unavailable;
            }
        };

        if payload.success != Some(true) {
            log::debug!(
                "GeoIpService: provider returned success=false for ip={}",
                ip
            );
            return GeoIpFetchResult::Unavailable;
        }

        log::debug!(
            "GeoIpService: provider returned successful geoip payload for ip={}",
            ip
        );
        GeoIpFetchResult::Success(GeoIpInfo {
            city: payload.city.unwrap_or_default(),
            region: payload.region.unwrap_or_default(),
            country: payload.country.unwrap_or_default(),
            country_code: payload.country_code.unwrap_or_default(),
            timezone: payload
                .timezone
                .and_then(|timezone| timezone.id)
                .unwrap_or_default(),
            org: payload
                .connection
                .and_then(|connection| connection.org.or(connection.isp))
                .unwrap_or_default(),
            flag_url: payload.flag.and_then(|flag| flag.img).unwrap_or_default(),
        })
    }

    #[cfg(test)]
    fn with_provider_base_url(mut self, provider_base_url: String) -> Self {
        self.provider_base_url = provider_base_url;
        self
    }
}

enum GeoIpFetchResult {
    Success(GeoIpInfo),
    RateLimited(Option<Duration>),
    Unavailable,
}

fn normalize_ip_for_lookup(raw: &str) -> Option<String> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(bracketed) = value.strip_prefix('[') {
        let (inner, maybe_port) = bracketed.split_once(']')?;
        if maybe_port.is_empty() || maybe_port.starts_with(':') {
            return Some(inner.to_string());
        }
    }

    if let Some((ip, _port)) = value.split_once(':') {
        if ip.split('.').count() == 4 {
            return Some(ip.to_string());
        }
    }

    Some(value.to_string())
}

fn is_public_ip(ip: &str) -> bool {
    let normalized = ip.to_lowercase();
    if normalized.is_empty()
        || normalized == "unknown"
        || normalized == "localhost"
        || normalized == "::1"
    {
        return false;
    }

    if normalized.contains(':') {
        return !(normalized.starts_with("fc")
            || normalized.starts_with("fd")
            || normalized.starts_with("fe80"));
    }

    let mut octets = [0_u8; 4];
    let parts: Vec<&str> = normalized.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    for (idx, part) in parts.iter().enumerate() {
        let parsed = match part.parse::<u8>() {
            Ok(value) => value,
            Err(_) => return false,
        };
        octets[idx] = parsed;
    }

    let first = octets[0];
    let second = octets[1];

    if first == 10 || first == 127 {
        return false;
    }
    if first == 172 && (16..=31).contains(&second) {
        return false;
    }
    if first == 192 && second == 168 {
        return false;
    }
    if first == 169 && second == 254 {
        return false;
    }
    if first == 100 && (64..=127).contains(&second) {
        return false;
    }
    if first >= 224 {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{GeoIpService, is_public_ip, normalize_ip_for_lookup};
    use httpmock::Method::GET;
    use httpmock::MockServer;
    use std::time::Duration;

    #[test]
    fn normalize_ip_for_lookup_handles_common_shapes() {
        let cases = [
            ("8.8.8.8", Some("8.8.8.8")),
            ("8.8.8.8:443", Some("8.8.8.8")),
            (" 8.8.8.8:53 ", Some("8.8.8.8")),
            ("[2001:db8::1]:443", Some("2001:db8::1")),
            ("2001:db8::1", Some("2001:db8::1")),
            ("", None),
            ("   ", None),
        ];

        for (input, expected) in cases {
            let normalized = normalize_ip_for_lookup(input);
            assert_eq!(normalized.as_deref(), expected);
        }
    }

    #[test]
    fn is_public_ip_filters_private_and_special_addresses() {
        let private_or_special = [
            "10.0.0.1",
            "127.0.0.1",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.1",
            "169.254.10.20",
            "100.64.0.1",
            "localhost",
            "::1",
            "unknown",
            "fc00::1",
            "fd12::abcd",
            "fe80::1",
        ];

        for ip in private_or_special {
            assert!(!is_public_ip(ip), "expected non-public ip: {ip}");
        }

        let public = ["8.8.8.8", "1.1.1.1", "2001:4860:4860::8888"];
        for ip in public {
            assert!(is_public_ip(ip), "expected public ip: {ip}");
        }
    }

    #[tokio::test]
    async fn lookup_ip_returns_none_when_service_disabled() {
        let service = GeoIpService::new(false);
        let result = service.lookup_ip("8.8.8.8").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn lookup_ip_returns_none_for_non_public_ip() {
        let service = GeoIpService::new(true);
        let result = service.lookup_ip("127.0.0.1").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn acquire_provider_slot_respects_min_interval() {
        let service = GeoIpService::new(true);

        assert!(service.acquire_provider_slot().await);
        assert!(!service.acquire_provider_slot().await);

        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert!(service.acquire_provider_slot().await);
    }

    #[tokio::test]
    async fn lookup_ip_caches_successful_provider_response() {
        let server = MockServer::start_async().await;
        let provider_mock = server
            .mock_async(|when, then| {
                when.method(GET).path("/8.8.8.8");
                then.status(200).json_body_obj(&serde_json::json!({
                    "success": true,
                    "country": "United States",
                    "country_code": "US",
                    "region": "California",
                    "city": "Mountain View",
                    "timezone": { "id": "America/Los_Angeles" },
                    "connection": { "org": "Google LLC" },
                    "flag": { "img": "https://example.com/flag.png" }
                }));
            })
            .await;

        let service = GeoIpService::new(true).with_provider_base_url(server.base_url().to_string());

        let first = service.lookup_ip("8.8.8.8").await;
        let second = service.lookup_ip("8.8.8.8").await;

        assert!(first.is_some());
        assert!(second.is_some());
        assert_eq!(provider_mock.hits_async().await, 1);
    }

    #[tokio::test]
    async fn lookup_ip_caches_unavailable_provider_response() {
        let server = MockServer::start_async().await;
        let provider_mock = server
            .mock_async(|when, then| {
                when.method(GET).path("/8.8.8.8");
                then.status(500);
            })
            .await;

        let service = GeoIpService::new(true).with_provider_base_url(server.base_url().to_string());

        let first = service.lookup_ip("8.8.8.8").await;
        let second = service.lookup_ip("8.8.8.8").await;

        assert!(first.is_none());
        assert!(second.is_none());
        assert_eq!(provider_mock.hits_async().await, 1);
    }
}
