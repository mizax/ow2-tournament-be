use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Organizer {
    pub role: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rules {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_rules_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_update: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscription {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitch_channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub donation_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub donation_amount_rub: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Eligibility {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_rank: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_competitive_hours: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_calibrated_seasons: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wins_current_season_main_role: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription: Option<Subscription>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_battletag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Checkin {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Registration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkin: Option<Checkin>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Teams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub players_per_team: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleItem {
    pub day: i32,
    pub date: NaiveDate,
    pub stage: String,
    pub start_time: String,
}

// Custom deserializer for MatchFormat to handle "final" field
#[derive(Debug, Serialize, Deserialize)]
pub struct MatchFormat {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playoff: Option<String>,
    #[serde(rename = "final")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_stage: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrizePlace {
    pub place: i32,
    pub amount: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrizePool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub places: Option<Vec<PrizePlace>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Stream {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum TournamentStatus {
    Draft,
    Upcoming,
    Ongoing,
    Finished,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TournamentResultPlace {
    pub place: i32,
    pub team_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captain_battletag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TournamentResults {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placements: Option<Vec<TournamentResultPlace>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mvp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TournamentMedia {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vod_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bracket_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Markdown {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_regulation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tournament {
    pub id: String,
    pub title: String,
    pub discipline: String,
    pub format: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organizers: Option<Vec<Organizer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Rules>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eligibility: Option<Eligibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration: Option<Registration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams: Option<Teams>,
    pub schedule: Vec<ScheduleItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_format: Option<MatchFormat>,
    pub prize_pool: PrizePool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<Stream>,
    pub status: TournamentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<TournamentResults>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<TournamentMedia>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<Markdown>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TournamentConfig {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organizers: Option<Vec<Organizer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Rules>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eligibility: Option<Eligibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration: Option<Registration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams: Option<Teams>,
    pub schedule: Vec<ScheduleItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_format: Option<MatchFormat>,
    pub prize_pool: PrizePool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<Stream>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<TournamentResults>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<TournamentMedia>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<Markdown>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TournamentShort {
    pub title: String,
    pub uri: String,
    pub discipline: String,
    pub format: String,
    pub dates: Vec<String>,
    pub prize_pool: Option<String>,
    pub registration_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub podium: Option<Vec<TournamentPodiumPlace>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TournamentPodiumPlace {
    pub place: i32,
    pub team_name: String,
}
