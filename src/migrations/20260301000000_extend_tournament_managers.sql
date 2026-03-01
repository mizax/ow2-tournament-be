ALTER TABLE tournament_managers ADD COLUMN is_owner INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tournament_managers ADD COLUMN can_manage_managers INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tournament_managers ADD COLUMN added_by_user_id INTEGER REFERENCES users(id);
-- SQLite does not allow non-constant defaults in ALTER TABLE ADD COLUMN,
-- so we use a placeholder literal and immediately backfill.
ALTER TABLE tournament_managers ADD COLUMN added_at DATETIME NOT NULL DEFAULT '1970-01-01 00:00:00';
UPDATE tournament_managers SET added_at = CURRENT_TIMESTAMP;

-- Backfill: mark owners from audit_log (tournament.created)
UPDATE tournament_managers
SET is_owner = 1, can_manage_managers = 1
WHERE (tournament_id, user_id) IN (
  SELECT entity_id, actor_user_id
  FROM audit_log
  WHERE action = 'tournament.created'
);

-- Backfill: give can_manage_managers to all remaining existing managers (backward compatibility)
UPDATE tournament_managers
SET can_manage_managers = 1
WHERE can_manage_managers = 0;
