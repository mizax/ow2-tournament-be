use chrono::NaiveDateTime;
use sqlx::{Pool, QueryBuilder, Sqlite};

#[derive(Clone)]
pub struct AuditDal {
    pool: Pool<Sqlite>,
}

pub struct AuditEntry {
    pub actor_user_id: i64,
    pub actor_battletag: String,
    pub action: &'static str,
    pub entity_type: &'static str,
    pub entity_id: i64,
    pub tournament_id: Option<i64>,
    pub details_json: String,
}

pub struct AuditFilter {
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
    pub tournament_id: Option<i64>,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditLogRow {
    pub id: i64,
    pub created_at: NaiveDateTime,
    pub actor_user_id: Option<i64>,
    pub actor_battletag: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: i64,
    pub tournament_id: Option<i64>,
    pub details_json: String,
}

impl AuditDal {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, entry: &AuditEntry) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO audit_log (
                actor_user_id,
                actor_battletag,
                action,
                entity_type,
                entity_id,
                tournament_id,
                details_json
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(entry.actor_user_id)
        .bind(&entry.actor_battletag)
        .bind(entry.action)
        .bind(entry.entity_type)
        .bind(entry.entity_id)
        .bind(entry.tournament_id)
        .bind(&entry.details_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list(&self, filter: AuditFilter) -> Result<(Vec<AuditLogRow>, i64), sqlx::Error> {
        let page = filter.page.max(1);
        let per_page = filter.per_page.max(1);
        let offset = (page - 1) * per_page;

        let mut total_builder = QueryBuilder::new(
            r#"
            SELECT COUNT(*)
            FROM audit_log
            WHERE 1 = 1
            "#,
        );
        if let Some(entity_type) = filter.entity_type.as_deref() {
            total_builder.push(" AND entity_type = ");
            total_builder.push_bind(entity_type);
        }
        if let Some(entity_id) = filter.entity_id {
            total_builder.push(" AND entity_id = ");
            total_builder.push_bind(entity_id);
        }
        if let Some(tournament_id) = filter.tournament_id {
            total_builder.push(" AND tournament_id = ");
            total_builder.push_bind(tournament_id);
        }
        let total = total_builder
            .build_query_scalar::<i64>()
            .fetch_one(&self.pool)
            .await?;

        let mut items_builder = QueryBuilder::new(
            r#"
            SELECT
                id,
                created_at,
                actor_user_id,
                actor_battletag,
                action,
                entity_type,
                entity_id,
                tournament_id,
                details_json
            FROM audit_log
            WHERE 1 = 1
            "#,
        );
        if let Some(entity_type) = filter.entity_type.as_deref() {
            items_builder.push(" AND entity_type = ");
            items_builder.push_bind(entity_type);
        }
        if let Some(entity_id) = filter.entity_id {
            items_builder.push(" AND entity_id = ");
            items_builder.push_bind(entity_id);
        }
        if let Some(tournament_id) = filter.tournament_id {
            items_builder.push(" AND tournament_id = ");
            items_builder.push_bind(tournament_id);
        }
        items_builder.push(" ORDER BY created_at DESC LIMIT ");
        items_builder.push_bind(per_page);
        items_builder.push(" OFFSET ");
        items_builder.push_bind(offset);

        let items = items_builder
            .build_query_as::<AuditLogRow>()
            .fetch_all(&self.pool)
            .await?;

        Ok((items, total))
    }
}
