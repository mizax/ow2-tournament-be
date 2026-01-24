ALTER TABLE tournaments RENAME COLUMN name TO title;
ALTER TABLE tournaments ADD COLUMN sef_title TEXT NOT NULL DEFAULT '';
ALTER TABLE tournaments ADD COLUMN discipline TEXT NOT NULL DEFAULT '';
ALTER TABLE tournaments ADD COLUMN format TEXT NOT NULL DEFAULT '';
ALTER TABLE tournaments ADD COLUMN dates_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE tournaments ADD COLUMN prize_pool_total_amount REAL NOT NULL DEFAULT 0;
ALTER TABLE tournaments ADD COLUMN prize_pool_currency TEXT NOT NULL DEFAULT '';

CREATE TABLE tournament_configuration (
  tournament_id INTEGER PRIMARY KEY,
  configuration_json TEXT NOT NULL,
  created_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  modified_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  FOREIGN KEY (tournament_id) REFERENCES tournaments(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_tournaments_sef_title ON tournaments (sef_title);
