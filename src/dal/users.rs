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

    pub async fn upsert_battletag(&self, user_id: i64, battletag: &str) -> Result<(), sqlx::Error> {
        self.upsert_battletag_with_executor(&self.pool, user_id, battletag)
            .await
    }

    pub async fn upsert_battletag_with_executor<'e, E>(
        &self,
        executor: E,
        user_id: i64,
        battletag: &str,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO user_battletags (user_id, battletag)
            VALUES (?1, ?2)
            "#,
        )
        .bind(user_id)
        .bind(battletag)
        .execute(executor)
        .await?;

        Ok(())
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
}
