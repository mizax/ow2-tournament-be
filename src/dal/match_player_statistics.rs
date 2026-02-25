use sqlx::{Executor, Pool, Sqlite};

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
        match_map_id: i64,
        round: i64,
        player_id: i64,
        hero_id: i64,
        stats: &[String],
    ) -> Result<(), sqlx::Error> {
        self.insert_with_executor(&self.pool, match_map_id, round, player_id, hero_id, stats)
            .await
    }

    pub async fn insert_with_executor<'e, E>(
        &self,
        executor: E,
        match_map_id: i64,
        round: i64,
        player_id: i64,
        hero_id: i64,
        stats: &[String],
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query(
            r#"
            INSERT INTO match_player_statistics (
              match_map_id, round, player_id, hero_id,
              eliminations, final_blows, deaths,
              all_damage, barrier_damage, hero_damage,
              healing_dealt, healing_received, self_healing,
              damage_taken, damage_blocked,
              defensive_assists, offensive_assists,
              ultimates_earned, ultimates_used,
              multikill_best, multikills, solo_kills,
              objective_kills, environmental_kills, environmental_deaths,
              critical_hits, critical_hit_accuracy,
              scoped_accuracy, scoped_critical_hit_accuracy, scoped_critical_hit_kills,
              shots_fired, shots_hit, shots_missed,
              scoped_shots_fired, scoped_shots_hit,
              weapon_accuracy, hero_time_played
            ) VALUES (
              ?1, ?2, ?3, ?4,
              ?5, ?6, ?7,
              ?8, ?9, ?10,
              ?11, ?12, ?13,
              ?14, ?15,
              ?16, ?17,
              ?18, ?19,
              ?20, ?21, ?22,
              ?23, ?24, ?25,
              ?26, ?27,
              ?28, ?29, ?30,
              ?31, ?32, ?33,
              ?34, ?35,
              ?36, ?37
            )
            "#,
        )
        .bind(match_map_id) // ?1
        .bind(round) // ?2
        .bind(player_id) // ?3
        .bind(hero_id) // ?4
        .bind(get_i64(stats, 0)) // ?5  eliminations
        .bind(get_i64(stats, 1)) // ?6  final_blows
        .bind(get_i64(stats, 2)) // ?7  deaths
        .bind(get_f64(stats, 3)) // ?8  all_damage
        .bind(get_f64(stats, 4)) // ?9  barrier_damage
        .bind(get_f64(stats, 5)) // ?10 hero_damage
        .bind(get_f64(stats, 6)) // ?11 healing_dealt
        .bind(get_f64(stats, 7)) // ?12 healing_received
        .bind(get_f64(stats, 8)) // ?13 self_healing
        .bind(get_f64(stats, 9)) // ?14 damage_taken
        .bind(get_f64(stats, 10)) // ?15 damage_blocked
        .bind(get_i64(stats, 11)) // ?16 defensive_assists
        .bind(get_i64(stats, 12)) // ?17 offensive_assists
        .bind(get_i64(stats, 13)) // ?18 ultimates_earned
        .bind(get_i64(stats, 14)) // ?19 ultimates_used
        .bind(get_i64(stats, 15)) // ?20 multikill_best
        .bind(get_i64(stats, 16)) // ?21 multikills
        .bind(get_i64(stats, 17)) // ?22 solo_kills
        .bind(get_i64(stats, 18)) // ?23 objective_kills
        .bind(get_i64(stats, 19)) // ?24 environmental_kills
        .bind(get_i64(stats, 20)) // ?25 environmental_deaths
        .bind(get_i64(stats, 21)) // ?26 critical_hits
        .bind(get_f64(stats, 22)) // ?27 critical_hit_accuracy
        .bind(get_f64(stats, 23)) // ?28 scoped_accuracy
        .bind(get_f64(stats, 24)) // ?29 scoped_critical_hit_accuracy
        .bind(get_i64(stats, 25)) // ?30 scoped_critical_hit_kills
        .bind(get_i64(stats, 26)) // ?31 shots_fired
        .bind(get_i64(stats, 27)) // ?32 shots_hit
        .bind(get_i64(stats, 28)) // ?33 shots_missed
        .bind(get_i64(stats, 29)) // ?34 scoped_shots_fired
        .bind(get_i64(stats, 30)) // ?35 scoped_shots_hit
        .bind(get_f64(stats, 31)) // ?36 weapon_accuracy
        .bind(get_f64(stats, 32)) // ?37 hero_time_played
        .execute(executor)
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
