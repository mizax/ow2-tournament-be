CREATE TABLE tournament_managers (
  tournament_id INTEGER NOT NULL,
  user_id INTEGER NOT NULL,
  PRIMARY KEY (tournament_id, user_id),
  FOREIGN KEY (tournament_id) REFERENCES tournaments(id) ON DELETE CASCADE,
  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_tournament_managers_user_id
  ON tournament_managers (user_id);
