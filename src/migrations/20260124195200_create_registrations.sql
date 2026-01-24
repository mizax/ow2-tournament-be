CREATE TABLE registrations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  tournament_id INTEGER NOT NULL,
  user_id INTEGER NOT NULL,
  user_battletag_id INTEGER NOT NULL,
  status TEXT NOT NULL DEFAULT 'PENDING',
  alt_accounts_json TEXT NOT NULL DEFAULT '[]',
  twitch TEXT NOT NULL,
  discord TEXT NOT NULL,
  primary_role TEXT,
  secondary_role TEXT,
  guarantors_json TEXT NOT NULL DEFAULT '[]',
  additional_info TEXT NOT NULL,
  rules_accepted INTEGER NOT NULL DEFAULT 0,
  ip_address TEXT NOT NULL,
  user_agent TEXT NOT NULL,
  decline_reason TEXT,
  created_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  updated_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  version INTEGER NOT NULL DEFAULT 1,

  FOREIGN KEY (tournament_id) REFERENCES tournaments(id) ON DELETE CASCADE,
  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
  FOREIGN KEY (user_battletag_id) REFERENCES user_battletags(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX uq_registrations_tournament_user_active
  ON registrations (tournament_id, user_id)
  WHERE status != 'DELETED';

CREATE INDEX idx_registrations_tournament_status
  ON registrations (tournament_id, status);
