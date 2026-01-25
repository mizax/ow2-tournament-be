use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Organizer {
    pub role: String,
    pub name: String,
    pub contact: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rules {
    pub full_rules_url: Option<String>,
    pub version: Option<String>,
    pub last_update: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscription {
    pub twitch_channel: Option<String>,
    pub donation_url: Option<String>,
    pub donation_amount_rub: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Eligibility {
    pub min_rank: Option<String>,
    pub min_competitive_hours: Option<f64>,
    pub min_calibrated_seasons: Option<f64>,
    pub wins_current_season_main_role: Option<f64>,
    pub subscription: Option<Subscription>,
    pub verification_battletag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Checkin {
    pub from: Option<String>,
    pub to: Option<String>,
    pub platform: Option<String>,
    pub platform_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Registration {
    pub start: Option<DateTime<Utc>>,
    pub deadline: Option<DateTime<Utc>>,
    pub checkin: Option<Checkin>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Teams {
    pub players_per_team: Option<f64>,
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
    pub group_stage: Option<String>,
    pub playoff: Option<String>,
    #[serde(rename = "final")]
    pub final_stage: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrizePlace {
    pub place: i32,
    pub amount: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrizePool {
    pub currency: Option<String>,
    pub places: Option<Vec<PrizePlace>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Stream {
    pub platform: Option<String>,
    pub channel: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Markdown {
    pub description: Option<String>,
    pub notes: Option<String>,
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
    pub organizers: Option<Vec<Organizer>>,
    pub rules: Option<Rules>,
    pub eligibility: Option<Eligibility>,
    pub registration: Option<Registration>,
    pub teams: Option<Teams>,
    pub schedule: Vec<ScheduleItem>,
    pub match_format: Option<MatchFormat>,
    pub prize_pool: PrizePool,
    pub stream: Option<Stream>,
    pub markdown: Option<Markdown>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TournamentConfig {
    #[serde(rename = "type")]
    pub type_: String,
    pub organizers: Option<Vec<Organizer>>,
    pub rules: Option<Rules>,
    pub eligibility: Option<Eligibility>,
    pub registration: Option<Registration>,
    pub teams: Option<Teams>,
    pub schedule: Vec<ScheduleItem>,
    pub match_format: Option<MatchFormat>,
    pub prize_pool: PrizePool,
    pub stream: Option<Stream>,
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
}
