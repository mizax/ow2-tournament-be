use sqlx::{Pool, Sqlite};

#[derive(Clone)]
pub struct MatchPlayerStatisticsRepo {
    pool: Pool<Sqlite>,
}

impl MatchPlayerStatisticsRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        match_id: i64,
        round: i64,
        player_id: i64,
        hero_id: i64,
        stats: &[String],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO match_player_statistics (
              match_id, round, player_id, hero_id,
              eliminations, final_blows, deaths, all_damage, barrier_damage, hero_damage,
              healing_dealt, healing_received, self_healing, damage_taken,
              defensive_assists, offensive_assists, ultimates_earned, ultimates_used,
              multikill_best, multikills, solo_kills, objective_kills,
              environmental_kills, environmental_deaths, critical_hits, critical_hit_accuracy,
              scoped_accuracy, scoped_critical_hit_accuracy, scoped_critical_hit_kills,
              shots_fired, shots_hit, shots_missed, scoped_shots_fired, scoped_shots_hit,
              weapon_accuracy, hero_time_played
            ) VALUES (
              ?1, ?2, ?3, ?4, ?5,
              ?6, ?7, ?8, ?9, ?10, ?11,
              ?12, ?13, ?14, ?15,
              ?16, ?17, ?18, ?19,
              ?20, ?21, ?22, ?23,
              ?24, ?25, ?26, ?27,
              ?28, ?29, ?30,
              ?31, ?32, ?33, ?34, ?35,
              ?36
            )
            "#,
        )
        .bind(match_id)
        .bind(round)
        .bind(player_id)
        .bind(hero_id)
        .bind(get_i64(stats, 0))
        .bind(get_i64(stats, 1))
        .bind(get_i64(stats, 2))
        .bind(get_f64(stats, 3))
        .bind(get_f64(stats, 4))
        .bind(get_f64(stats, 5))
        .bind(get_f64(stats, 6))
        .bind(get_f64(stats, 7))
        .bind(get_f64(stats, 8))
        .bind(get_f64(stats, 9))
        .bind(get_i64(stats, 10))
        .bind(get_i64(stats, 11))
        .bind(get_i64(stats, 12))
        .bind(get_i64(stats, 13))
        .bind(get_i64(stats, 14))
        .bind(get_i64(stats, 15))
        .bind(get_i64(stats, 16))
        .bind(get_i64(stats, 17))
        .bind(get_i64(stats, 18))
        .bind(get_i64(stats, 19))
        .bind(get_i64(stats, 20))
        .bind(get_f64(stats, 21))
        .bind(get_f64(stats, 22))
        .bind(get_f64(stats, 23))
        .bind(get_i64(stats, 24))
        .bind(get_i64(stats, 25))
        .bind(get_i64(stats, 26))
        .bind(get_i64(stats, 27))
        .bind(get_i64(stats, 28))
        .bind(get_i64(stats, 29))
        .bind(get_f64(stats, 30))
        .bind(get_f64(stats, 31))
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

fn get_i64(stats: &[String], index: usize) -> i64 {
    stats
        .get(index)
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(0)
}

fn get_f64(stats: &[String], index: usize) -> f64 {
    stats
        .get(index)
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(0.0)
}
