CREATE TABLE audit_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  created_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  actor_user_id INTEGER,
  actor_battletag TEXT NOT NULL DEFAULT '',
  action TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id INTEGER NOT NULL,
  tournament_id INTEGER,
  details_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX idx_audit_log_entity
  ON audit_log (entity_type, entity_id, created_at DESC);

CREATE INDEX idx_audit_log_tournament
  ON audit_log (tournament_id, created_at DESC);

CREATE INDEX idx_audit_log_created
  ON audit_log (created_at DESC);
