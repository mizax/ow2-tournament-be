CREATE TABLE user_authorities (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  user_id INTEGER NOT NULL,
  authority TEXT NOT NULL,
  granted_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  granted_by_user_id INTEGER,
  UNIQUE(user_id, authority),
  FOREIGN KEY (user_id) REFERENCES users(id),
  FOREIGN KEY (granted_by_user_id) REFERENCES users(id)
);

CREATE INDEX idx_user_authorities_user_id ON user_authorities (user_id);
