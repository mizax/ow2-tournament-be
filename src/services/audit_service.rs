use crate::dal::{AuditEntry, Dal};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct AuditActor {
    pub user_id: i64,
    pub battletag: String,
}

#[derive(Debug, Clone, Copy)]
pub struct AuditKind {
    pub action: &'static str,
    pub entity_type: &'static str,
}

pub type ChangedFields = serde_json::Map<String, Value>;

pub mod kinds {
    use super::AuditKind;

    pub const TOURNAMENT_UPDATED: AuditKind = AuditKind {
        action: "tournament.updated",
        entity_type: "tournament",
    };

    pub const REGISTRATION_STATUS_CHANGED: AuditKind = AuditKind {
        action: "registration.status_changed",
        entity_type: "registration",
    };

    pub const REGISTRATION_ROLE_RANKINGS_UPDATED: AuditKind = AuditKind {
        action: "registration.role_rankings_updated",
        entity_type: "registration",
    };

    pub const USER_AUTHORITY_GRANTED: AuditKind = AuditKind {
        action: "user.authority_granted",
        entity_type: "user",
    };

    pub const USER_AUTHORITY_REVOKED: AuditKind = AuditKind {
        action: "user.authority_revoked",
        entity_type: "user",
    };

    pub const TOURNAMENT_CREATED: AuditKind = AuditKind {
        action: "tournament.created",
        entity_type: "tournament",
    };
}

pub fn insert_change_if_changed(
    changed_fields: &mut ChangedFields,
    field: &str,
    old: Value,
    new: Value,
) {
    if old != new {
        changed_fields.insert(field.to_string(), change(old, new));
    }
}

pub fn change(old: Value, new: Value) -> Value {
    serde_json::json!({ "old": old, "new": new })
}

pub fn insert_serialized_change_if_changed<T: serde::Serialize>(
    changed_fields: &mut ChangedFields,
    field: &str,
    old: &T,
    new: &T,
) {
    let old_value = serde_json::to_value(old).unwrap_or(Value::Null);
    let new_value = serde_json::to_value(new).unwrap_or(Value::Null);
    insert_change_if_changed(changed_fields, field, old_value, new_value);
}

pub fn diff_json_objects(old: &Value, new: &Value) -> ChangedFields {
    let mut changed = ChangedFields::new();
    let empty = serde_json::Map::new();
    let old_obj = old.as_object().unwrap_or(&empty);
    let new_obj = new.as_object().unwrap_or(&empty);
    let all_keys = old_obj
        .keys()
        .chain(new_obj.keys())
        .collect::<std::collections::BTreeSet<_>>();

    for key in all_keys {
        let old_val = old_obj.get(key).cloned().unwrap_or(Value::Null);
        let new_val = new_obj.get(key).cloned().unwrap_or(Value::Null);
        if old_val != new_val {
            changed.insert(key.clone(), change(old_val, new_val));
        }
    }

    changed
}

pub async fn log_event(
    db: &Dal,
    actor: &AuditActor,
    kind: AuditKind,
    entity_id: i64,
    tournament_id: Option<i64>,
    details: Value,
) {
    let details_json = serde_json::to_string(&details).unwrap_or_else(|_| "{}".to_string());
    if let Err(error) = db
        .audit
        .insert(&AuditEntry {
            actor_user_id: actor.user_id,
            actor_battletag: actor.battletag.clone(),
            action: kind.action,
            entity_type: kind.entity_type,
            entity_id,
            tournament_id,
            details_json,
        })
        .await
    {
        log::error!(
            "Audit insert failed for action {} entity {}#{}: {}",
            kind.action,
            kind.entity_type,
            entity_id,
            error
        );
    }
}
