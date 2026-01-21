use crate::api::error::ApiError;
use actix_web::{HttpResponse, Responder, get, web};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use serde_json::json;

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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Registration {
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
pub struct TournamentShort<'a> {
    title: &'a str,
    uri: &'a str,
    dates: Vec<&'a str>,
    prize_pool: Option<&'a str>,
}

#[get("")]
pub async fn get_tournaments() -> actix_web::Result<impl Responder, ApiError> {
    Ok(HttpResponse::Ok().json(json!(vec![
        TournamentShort {
            title: "День Защитника Пейлоада",
            uri: "2026-02-defender-of-the-payload-day",
            dates: vec!["2026-02-21", "2026-02-22"],
            prize_pool: Some("40000 RUB")
        }
    ])))
}

#[get("/{id}")]
pub async fn get_tournament(id: web::Path<String>) -> actix_web::Result<impl Responder, ApiError> {
    let id = id.into_inner();
    
    // Basic path traversal protection
    if id.contains("..") || id.contains('/') || id.contains('\\') {
        return Err(ApiError::BadRequest {
            error: "Invalid ID".to_string(),
            details: "ID contains invalid characters".to_string(),
        });
    }

    let mut path = PathBuf::from("./static/tournaments");
    path.push(format!("{}.json", id));

    match fs::read_to_string(path) {
        Ok(content) => {
            let tournament: Tournament = serde_json::from_str(&content).map_err(|e| {
                log::error!("Failed to parse tournament JSON: {}", e);
                ApiError::InternalError {
                    error: e.to_string(),
                }
            })?;
            Ok(HttpResponse::Ok().json(tournament))
        }
        Err(_) => Err(ApiError::NotFound),
    }
}
