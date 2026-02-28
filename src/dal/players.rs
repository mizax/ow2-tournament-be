use sqlx::{Executor, Pool, Sqlite};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PlayerProfileRow {
    pub id: i64,
    pub nickname: String,
    pub role: Option<String>,
    pub registration_id: Option<i64>,
    pub battletag: Option<String>,
    pub team_name: String,
    pub tournament_title: String,
    pub tournament_sef: String,
    pub division_name: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PlayerMatchSummaryRow {
    pub match_id: i64,
    pub home_score: Option<i64>,
    pub away_score: Option<i64>,
    pub home_team: String,
    pub away_team: String,
    pub tournament_title: String,
    pub maps_played: i64,
    pub kills: Option<i64>,
    pub deaths: Option<i64>,
    pub damage: Option<f64>,
    pub healing: Option<f64>,
    pub time_played: Option<f64>,
}

#[derive(Clone)]
pub struct PlayersRepo {
    pool: Pool<Sqlite>,
}

impl PlayersRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn find_team_id_by_tournament_and_nickname(
        &self,
        tournament_id: i64,
        nickname: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        self.find_team_id_by_tournament_and_nickname_with_executor(
            &self.pool,
            tournament_id,
            nickname,
        )
        .await
    }

    pub async fn find_team_id_by_tournament_and_nickname_with_executor<'e, E>(
        &self,
        executor: E,
        tournament_id: i64,
        nickname: &str,
    ) -> Result<Option<i64>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT p.team_id
            FROM players p
            JOIN teams t ON t.id = p.team_id
            WHERE t.tournament_id = ?1 AND p.nickname = ?2
            LIMIT 1
            "#,
        )
        .bind(tournament_id)
        .bind(nickname)
        .fetch_optional(executor)
        .await
    }

    pub async fn find_id_by_team_and_nickname(
        &self,
        team_id: i64,
        nickname: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        self.find_id_by_team_and_nickname_with_executor(&self.pool, team_id, nickname)
            .await
    }

    pub async fn find_id_by_team_and_nickname_with_executor<'e, E>(
        &self,
        executor: E,
        team_id: i64,
        nickname: &str,
    ) -> Result<Option<i64>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT id FROM players WHERE team_id = ?1 AND nickname = ?2"#,
        )
        .bind(team_id)
        .bind(nickname)
        .fetch_optional(executor)
        .await
    }

    pub async fn create(&self, team_id: i64, nickname: &str) -> Result<i64, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(r#"INSERT INTO players (team_id, nickname) VALUES (?1, ?2)"#)
            .bind(team_id)
            .bind(nickname)
            .execute(&mut *tx)
            .await?;

        let id = sqlx::query_scalar::<_, i64>(r#"SELECT last_insert_rowid()"#)
            .fetch_one(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(id)
    }

    pub async fn create_with_executor<'e, E>(
        &self,
        executor: &mut E,
        team_id: i64,
        nickname: &str,
    ) -> Result<i64, sqlx::Error>
    where
        for<'c> &'c mut E: Executor<'c, Database = Sqlite>,
    {
        sqlx::query(r#"INSERT INTO players (team_id, nickname) VALUES (?1, ?2)"#)
            .bind(team_id)
            .bind(nickname)
            .execute(&mut *executor)
            .await?;

        sqlx::query_scalar::<_, i64>(r#"SELECT last_insert_rowid()"#)
            .fetch_one(&mut *executor)
            .await
    }

    pub async fn get_public_profile(
        &self,
        player_id: i64,
    ) -> Result<Option<PlayerProfileRow>, sqlx::Error> {
        sqlx::query_as::<_, PlayerProfileRow>(
            r#"
            SELECT p.id, p.nickname, p.role, p.registration_id,
                   t.name as team_name,
                   tour.title as tournament_title, tour.sef_title as tournament_sef,
                   d.name as division_name,
                   ub.battletag
            FROM players p
            JOIN teams t ON p.team_id = t.id
            JOIN tournaments tour ON t.tournament_id = tour.id
            LEFT JOIN divisions d ON p.division_id = d.id
            LEFT JOIN registrations r ON p.registration_id = r.id
            LEFT JOIN user_battletags ub ON r.user_battletag_id = ub.id
            WHERE p.id = ?1 AND p.deleted_at IS NULL
            "#,
        )
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn list_public_match_summaries(
        &self,
        player_id: i64,
    ) -> Result<Vec<PlayerMatchSummaryRow>, sqlx::Error> {
        sqlx::query_as::<_, PlayerMatchSummaryRow>(
            r#"
            SELECT m.id as match_id, m.home_score, m.away_score,
                   ht.name as home_team, at.name as away_team,
                   tour.title as tournament_title,
                   COUNT(DISTINCT mps.match_map_id) as maps_played,
                   SUM(mps.eliminations) as kills,
                   SUM(mps.deaths) as deaths,
                   SUM(mps.all_damage) as damage,
                   SUM(mps.healing_dealt) as healing,
                   SUM(mps.hero_time_played) as time_played
            FROM match_player_statistics mps
            JOIN match_maps mm ON mps.match_map_id = mm.id
            JOIN matches m ON mm.match_id = m.id
            JOIN teams ht ON m.home_team_id = ht.id
            JOIN teams at ON m.away_team_id = at.id
            JOIN tournaments tour ON m.tournament_id = tour.id
            WHERE mps.player_id = ?1
            GROUP BY m.id
            ORDER BY m.id DESC
            "#,
        )
        .bind(player_id)
        .fetch_all(&self.pool)
        .await
    }
}
