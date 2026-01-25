CREATE TABLE registration_comments (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  registration_id INTEGER NOT NULL,
  manager_user_id INTEGER NOT NULL,
  comment TEXT NOT NULL,
  created_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),

  FOREIGN KEY (registration_id) REFERENCES registrations(id) ON DELETE CASCADE,
  FOREIGN KEY (manager_user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_registration_comments_registration_id_created_at
  ON registration_comments (registration_id, created_at);

CREATE TABLE registration_requested_actions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  registration_id INTEGER NOT NULL,
  manager_user_id INTEGER NOT NULL,
  description TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'PENDING',
  created_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  updated_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),

  CHECK (status IN ('PENDING', 'RESOLVED')),
  FOREIGN KEY (registration_id) REFERENCES registrations(id) ON DELETE CASCADE,
  FOREIGN KEY (manager_user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_registration_requested_actions_registration_id
  ON registration_requested_actions (registration_id);

CREATE INDEX idx_registration_requested_actions_status
  ON registration_requested_actions (status);

CREATE TABLE registration_role_rankings (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  registration_id INTEGER NOT NULL,
  role TEXT NOT NULL,
  ranking INTEGER NOT NULL,
  created_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),

  CHECK (role IN ('TANK', 'DAMAGE', 'SUPPORT', 'FLEX')),
  CHECK (ranking >= 0),
  UNIQUE (registration_id, role),
  FOREIGN KEY (registration_id) REFERENCES registrations(id) ON DELETE CASCADE
);

CREATE INDEX idx_registration_role_rankings_registration_id
  ON registration_role_rankings (registration_id);
