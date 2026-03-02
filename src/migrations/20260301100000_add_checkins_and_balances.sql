CREATE TABLE registration_checkins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tournament_id INTEGER NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    registration_id INTEGER NOT NULL UNIQUE REFERENCES registrations(id) ON DELETE CASCADE,
    checked_in INTEGER NOT NULL DEFAULT 0,
    checked_in_at TEXT,
    updated_at TEXT NOT NULL,
    updated_by_user_id INTEGER REFERENCES users(id),
    primary_role_override TEXT,
    secondary_role_override TEXT,
    role_rankings_override_json TEXT,
    full_flex_override INTEGER
);
CREATE INDEX idx_checkins_tournament_id ON registration_checkins(tournament_id);

CREATE TABLE tournament_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tournament_id INTEGER NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    created_by INTEGER NOT NULL REFERENCES users(id),
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_balances_tournament_id ON tournament_balances(tournament_id);
