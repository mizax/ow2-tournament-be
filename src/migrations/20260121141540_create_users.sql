CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  created_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  updated_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  is_admin INTEGER NOT NULL DEFAULT 0,
  is_banned INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE user_battletags (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id INTEGER NOT NULL,
  battletag TEXT NOT NULL,
  created_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  CONSTRAINT fk_user_battletags_user_id
    FOREIGN KEY(user_id) REFERENCES users(id)
    ON DELETE CASCADE,
  CONSTRAINT uq_user_battletags_user_id_battletag
    UNIQUE(user_id, battletag)
);
