use chrono::NaiveDateTime;
use sqlx::{Executor, Pool, Sqlite};

#[derive(Clone)]
pub struct UsersRepo {
    pool: Pool<Sqlite>,
}

#[derive(sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub is_admin: bool,
    pub is_banned: bool,
}

#[derive(sqlx::FromRow)]
pub struct UserBattletag {
    pub id: i64,
    pub user_id: i64,
    pub battletag: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserAuthorityRow {
    pub user_id: i64,
    pub authority: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserWithBattletagRow {
    pub id: i64,
    pub is_admin: bool,
    pub is_banned: bool,
    pub battletag: Option<String>,
    pub authorities_json: String,
}

impl UsersRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, user_id: i64) -> Result<Option<User>, sqlx::Error> {
        self.find_by_id_with_executor(&self.pool, user_id).await
    }

    pub async fn find_by_id_with_executor<'e, E>(
        &self,
        executor: E,
        user_id: i64,
    ) -> Result<Option<User>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as::<_, User>(
            r#"
            SELECT
                id,
                created_at,
                updated_at,
                is_admin,
                is_banned
            FROM users
            WHERE id = ?1
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(executor)
        .await
    }

    pub async fn create_user(&self, user_id: i64) -> Result<User, sqlx::Error> {
        sqlx::query(r#"INSERT INTO users (id) VALUES (?1)"#)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        self.find_by_id(user_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn create_user_with_executor<'e, E>(
        &self,
        executor: &mut E,
        user_id: i64,
    ) -> Result<User, sqlx::Error>
    where
        for<'c> &'c mut E: Executor<'c, Database = Sqlite>,
    {
        sqlx::query(r#"INSERT INTO users (id) VALUES (?1)"#)
            .bind(user_id)
            .execute(&mut *executor)
            .await?;

        sqlx::query_as::<_, User>(
            r#"
            SELECT
                id,
                created_at,
                updated_at,
                is_admin,
                is_banned
            FROM users
            WHERE id = ?1
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_one(&mut *executor)
        .await
    }

    pub async fn battletag_exists(
        &self,
        user_id: i64,
        battletag: &str,
    ) -> Result<bool, sqlx::Error> {
        self.battletag_exists_with_executor(&self.pool, user_id, battletag)
            .await
    }

    pub async fn battletag_exists_with_executor<'e, E>(
        &self,
        executor: E,
        user_id: i64,
        battletag: &str,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT 1
            FROM user_battletags
            WHERE user_id = ?1 AND battletag = ?2
            LIMIT 1
        "#,
        )
        .bind(user_id)
        .bind(battletag)
        .fetch_optional(executor)
        .await?
        .is_some();

        Ok(exists)
    }

    pub async fn insert_battletag(&self, user_id: i64, battletag: &str) -> Result<(), sqlx::Error> {
        self.insert_battletag_with_executor(&self.pool, user_id, battletag)
            .await
    }

    pub async fn insert_battletag_with_executor<'e, E>(
        &self,
        executor: E,
        user_id: i64,
        battletag: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query(r#"INSERT INTO user_battletags (user_id, battletag) VALUES (?1, ?2)"#)
            .bind(user_id)
            .bind(battletag)
            .execute(executor)
            .await?;

        Ok(())
    }

    pub async fn upsert_battletag(&self, user_id: i64, battletag: &str) -> Result<bool, sqlx::Error> {
        self.upsert_battletag_with_executor(&self.pool, user_id, battletag)
            .await
    }

    pub async fn upsert_battletag_with_executor<'e, E>(
        &self,
        executor: E,
        user_id: i64,
        battletag: &str,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO user_battletags (user_id, battletag)
            VALUES (?1, ?2)
            "#,
        )
        .bind(user_id)
        .bind(battletag)
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn find_battletag_id(
        &self,
        user_id: i64,
        battletag: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        self.find_battletag_id_with_executor(&self.pool, user_id, battletag)
            .await
    }

    pub async fn find_battletag_id_with_executor<'e, E>(
        &self,
        executor: E,
        user_id: i64,
        battletag: &str,
    ) -> Result<Option<i64>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id
            FROM user_battletags
            WHERE user_id = ?1 AND battletag = ?2
            LIMIT 1
        "#,
        )
        .bind(user_id)
        .bind(battletag)
        .fetch_optional(executor)
        .await
    }

    pub async fn find_battletag_by_id(
        &self,
        user_battletag_id: i64,
    ) -> Result<Option<UserBattletag>, sqlx::Error> {
        self.find_battletag_by_id_with_executor(&self.pool, user_battletag_id)
            .await
    }

    pub async fn find_battletag_by_id_with_executor<'e, E>(
        &self,
        executor: E,
        user_battletag_id: i64,
    ) -> Result<Option<UserBattletag>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as::<_, UserBattletag>(
            r#"
            SELECT
                id,
                user_id,
                battletag,
                created_at
            FROM user_battletags
            WHERE id = ?1
            LIMIT 1
        "#,
        )
        .bind(user_battletag_id)
        .fetch_optional(executor)
        .await
    }

    pub async fn touch_updated_at(&self, user_id: i64) -> Result<(), sqlx::Error> {
        self.touch_updated_at_with_executor(&self.pool, user_id)
            .await
    }

    pub async fn touch_updated_at_with_executor<'e, E>(
        &self,
        executor: E,
        user_id: i64,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query(r#"UPDATE users SET updated_at = CURRENT_TIMESTAMP WHERE id = ?1"#)
            .bind(user_id)
            .execute(executor)
            .await?;

        Ok(())
    }

    pub async fn list_authorities_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<String>, sqlx::Error> {
        self.list_authorities_for_user_with_executor(&self.pool, user_id)
            .await
    }

    pub async fn list_authorities_for_user_with_executor<'e, E>(
        &self,
        executor: E,
        user_id: i64,
    ) -> Result<Vec<String>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let rows = sqlx::query_as::<_, UserAuthorityRow>(
            r#"
            SELECT
                user_id,
                authority
            FROM user_authorities
            WHERE user_id = ?1
            ORDER BY authority
            "#,
        )
        .bind(user_id)
        .fetch_all(executor)
        .await?;

        Ok(rows.into_iter().map(|row| row.authority).collect())
    }

    pub async fn grant_authority(
        &self,
        user_id: i64,
        authority: &str,
        granted_by_user_id: i64,
    ) -> Result<(), sqlx::Error> {
        self.grant_authority_with_executor(&self.pool, user_id, authority, granted_by_user_id)
            .await
    }

    pub async fn grant_authority_with_executor<'e, E>(
        &self,
        executor: E,
        user_id: i64,
        authority: &str,
        granted_by_user_id: i64,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO user_authorities (user_id, authority, granted_by_user_id)
            VALUES (?1, ?2, ?3)
            "#,
        )
        .bind(user_id)
        .bind(authority)
        .bind(granted_by_user_id)
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn revoke_authority(&self, user_id: i64, authority: &str) -> Result<(), sqlx::Error> {
        self.revoke_authority_with_executor(&self.pool, user_id, authority)
            .await
    }

    pub async fn revoke_authority_with_executor<'e, E>(
        &self,
        executor: E,
        user_id: i64,
        authority: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query(
            r#"
            DELETE FROM user_authorities
            WHERE user_id = ?1
              AND authority = ?2
            "#,
        )
        .bind(user_id)
        .bind(authority)
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn list_with_battletags(
        &self,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserWithBattletagRow>, sqlx::Error> {
        self.list_with_battletags_with_executor(&self.pool, search, limit, offset)
            .await
    }

    pub async fn list_with_battletags_with_executor<'e, E>(
        &self,
        executor: E,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserWithBattletagRow>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let trimmed_search = search.map(str::trim).filter(|v| !v.is_empty());
        let like = trimmed_search.map(|s| format!("%{}%", s));

        sqlx::query_as::<_, UserWithBattletagRow>(
            r#"
            SELECT
                u.id,
                u.is_admin,
                u.is_banned,
                ub.battletag,
                (
                    SELECT COALESCE(json_group_array(ua.authority), '[]')
                    FROM user_authorities ua
                    WHERE ua.user_id = u.id
                ) AS authorities_json
            FROM users u
            LEFT JOIN user_battletags ub
                ON ub.id = (
                    SELECT ub2.id
                    FROM user_battletags ub2
                    WHERE ub2.user_id = u.id
                    ORDER BY ub2.id DESC
                    LIMIT 1
                )
            WHERE (?1 IS NULL OR ub.battletag LIKE ?2)
            ORDER BY u.id DESC
            LIMIT ?3 OFFSET ?4
            "#,
        )
        .bind(trimmed_search)
        .bind(like)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }
}
