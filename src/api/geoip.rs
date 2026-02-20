use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

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
    client: reqwest::Client,
    state: std::sync::Arc<Mutex<GeoIpState>>,
}

#[derive(Default)]
struct GeoIpState {
    cache: HashMap<String, CachedGeoIp>,
    recent_requests: VecDeque<Instant>,
    last_request_at: Option<Instant>,
    blocked_until: Option<Instant>,
}

struct CachedGeoIp {
    value: Option<GeoIpInfo>,
    expires_at: Instant,
}

impl GeoIpService {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            client: reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()
                .expect("geoip http client must be constructable"),
            state: std::sync::Arc::new(Mutex::new(GeoIpState::default())),
        }
    }

    pub async fn lookup_ip(&self, raw_ip: &str) -> Option<GeoIpInfo> {
        if !self.enabled {
            return None;
        }

        let ip = normalize_ip_for_lookup(raw_ip)?;
        if !is_public_ip(&ip) {
            return None;
        }

        if let Some(cached) = self.get_cached(&ip).await {
            return cached;
        }

        if !self.acquire_provider_slot().await {
            return None;
        }

        match self.fetch_provider_geoip(&ip).await {
            GeoIpFetchResult::Success(value) => {
                self.insert_cache(ip, Some(value.clone()), SUCCESS_CACHE_TTL)
                    .await;
                Some(value)
            }
            GeoIpFetchResult::RateLimited(retry_after) => {
                self.apply_provider_block(retry_after).await;
                self.insert_cache(ip, None, FAILED_CACHE_TTL).await;
                None
            }
            GeoIpFetchResult::Unavailable => {
                self.insert_cache(ip, None, FAILED_CACHE_TTL).await;
                None
            }
        }
    }

    async fn get_cached(&self, ip: &str) -> Option<Option<GeoIpInfo>> {
        let mut state = self.state.lock().await;
        match state.cache.get(ip) {
            Some(cached) if cached.expires_at > Instant::now() => Some(cached.value.clone()),
            Some(_) => {
                state.cache.remove(ip);
                None
            }
            None => None,
        }
    }

    async fn insert_cache(&self, ip: String, value: Option<GeoIpInfo>, ttl: Duration) {
        let mut state = self.state.lock().await;
        state.cache.insert(
            ip,
            CachedGeoIp {
                value,
                expires_at: Instant::now() + ttl,
            },
        );
    }

    async fn acquire_provider_slot(&self) -> bool {
        let now = Instant::now();
        let mut state = self.state.lock().await;

        if state.blocked_until.is_some_and(|until| until > now) {
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
            return false;
        }

        if state
            .last_request_at
            .is_some_and(|stamp| now.duration_since(stamp) < MIN_REQUEST_INTERVAL)
        {
            return false;
        }

        state.last_request_at = Some(now);
        state.recent_requests.push_back(now);
        true
    }

    async fn apply_provider_block(&self, retry_after: Option<Duration>) {
        let block_for = retry_after.unwrap_or(PROVIDER_BLOCK_FALLBACK);
        let mut state = self.state.lock().await;
        state.blocked_until = Some(Instant::now() + block_for);
    }

    async fn fetch_provider_geoip(&self, ip: &str) -> GeoIpFetchResult {
        let response = match self
            .client
            .get(format!("{IPWHO_BASE_URL}/{ip}"))
            .send()
            .await
        {
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
            return GeoIpFetchResult::Unavailable;
        }

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
