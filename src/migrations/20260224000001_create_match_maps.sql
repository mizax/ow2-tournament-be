-- Maps fixtures (canonical English names)
INSERT INTO maps (name) VALUES
  ('Aatlis'),
  ('Blizzard World'),
  ('Busan'),
  ('Circuit Royal'),
  ('Colosseo'),
  ('Eichenwalde'),
  ('Havana'),
  ('Ilios'),
  ('Junkertown'),
  ("King's Row"),
  ('Lijiang Tower'),
  ('Midtown'),
  ('Nepal'),
  ('New Junk City'),
  ('Numbani'),
  ('Route 66'),
  ('Runasapi'),
  ('Shambali Monastery'),
  ('Watchpoint: Gibraltar'),
  ('Esperança');

-- Modes fixtures
INSERT INTO modes (name) VALUES
  ('Control'),
  ('Escort'),
  ('Flashpoint'),
  ('Hybrid'),
  ('Push');

-- game_maps fixtures (map + mode combinations)
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Aatlis'                 AND mo.name = 'Flashpoint';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Blizzard World'         AND mo.name = 'Hybrid';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Busan'                  AND mo.name = 'Control';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Circuit Royal'          AND mo.name = 'Escort';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Colosseo'               AND mo.name = 'Push';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Eichenwalde'            AND mo.name = 'Hybrid';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Havana'                 AND mo.name = 'Escort';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Ilios'                  AND mo.name = 'Control';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Junkertown'             AND mo.name = 'Escort';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'King''s Row'            AND mo.name = 'Hybrid';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Lijiang Tower'          AND mo.name = 'Control';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Midtown'                AND mo.name = 'Hybrid';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Nepal'                  AND mo.name = 'Control';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'New Junk City'          AND mo.name = 'Flashpoint';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Numbani'                AND mo.name = 'Hybrid';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Route 66'               AND mo.name = 'Escort';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Runasapi'               AND mo.name = 'Push';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Shambali Monastery'     AND mo.name = 'Escort';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Watchpoint: Gibraltar'  AND mo.name = 'Escort';
INSERT INTO game_maps (map_id, mode_id)
SELECT m.id, mo.id FROM maps m, modes mo WHERE m.name = 'Esperança'              AND mo.name = 'Push';

-- map_name_aliases: локализованные и сезонные названия -> canonical game_map_id
-- Используется при загрузке логов для поиска game_map_id по названию из лога
CREATE TABLE map_name_aliases (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  alias TEXT NOT NULL UNIQUE,
  game_map_id INTEGER NOT NULL,
  FOREIGN KEY (game_map_id) REFERENCES game_maps(id)
);

INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Aatlis',                       id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Aatlis');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Атлус',                        id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Aatlis');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Blizzard World',               id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Blizzard World');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Blizzard World (Winter)',      id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Blizzard World');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Busan',                        id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Busan');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Пусан',                        id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Busan');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Circuit royal',                id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Circuit Royal');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Circuit Royal',                id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Circuit Royal');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Colosseo',                     id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Colosseo');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Колизей',                      id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Colosseo');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Eichenwalde',                  id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Eichenwalde');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Havana',                       id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Havana');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Гавана',                       id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Havana');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Ilios',                        id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Ilios');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Илиос',                        id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Ilios');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Junkertown',                   id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Junkertown');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Джанкертаун',                  id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Junkertown');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'King''s Row',                  id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'King''s Row');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Кингс Роу (зима)',             id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'King''s Row');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Lijiang Tower',                id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Lijiang Tower');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Midtown',                      id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Midtown');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Мидтаун',                      id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Midtown');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Nepal',                        id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Nepal');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Непал',                        id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Nepal');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'New Junk City',                id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'New Junk City');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Нью-Джанк',                    id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'New Junk City');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Numbani',                      id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Numbani');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Route 66',                     id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Route 66');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Шоссе 66',                     id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Route 66');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Королевская трасса',           id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Route 66');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Runasapi',                     id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Runasapi');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Рунасапи',                     id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Runasapi');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Shambali Monastery',           id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Shambali Monastery');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Watchpoint: Gibraltar',        id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Watchpoint: Gibraltar');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Пост наблюдения: Гибралтар',   id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Watchpoint: Gibraltar');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Esperança',                    id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Esperança');
INSERT INTO map_name_aliases (alias, game_map_id) SELECT 'Эсперанса',                    id FROM game_maps WHERE map_id = (SELECT id FROM maps WHERE name = 'Esperança');

-- match_maps: одна запись на карту в серии
CREATE TABLE match_maps (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  match_id INTEGER NOT NULL,
  game_map_id INTEGER NOT NULL,
  map_order INTEGER NOT NULL,
  home_score INTEGER NOT NULL DEFAULT 0,
  away_score INTEGER NOT NULL DEFAULT 0,
  log_name TEXT,
  created_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  modified_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  UNIQUE (match_id, map_order),
  FOREIGN KEY (match_id) REFERENCES matches(id) ON DELETE NO ACTION ON UPDATE NO ACTION,
  FOREIGN KEY (game_map_id) REFERENCES game_maps(id) ON DELETE NO ACTION ON UPDATE NO ACTION
);

-- match_player_statistics: переключаем с match_id на match_map_id
CREATE TABLE match_player_statistics_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  match_map_id INTEGER NOT NULL,
  round INTEGER,
  player_id INTEGER,
  hero_id INTEGER,

  eliminations INTEGER,
  final_blows INTEGER,
  deaths INTEGER,
  all_damage FLOAT,
  barrier_damage FLOAT,
  hero_damage FLOAT,
  healing_dealt FLOAT,
  healing_received FLOAT,
  self_healing FLOAT,
  damage_taken FLOAT,
  defensive_assists INTEGER,
  offensive_assists INTEGER,
  ultimates_earned INTEGER,
  ultimates_used INTEGER,
  multikill_best INTEGER,
  multikills INTEGER,
  solo_kills INTEGER,
  objective_kills INTEGER,
  environmental_kills INTEGER,
  environmental_deaths INTEGER,
  critical_hits INTEGER,
  critical_hit_accuracy INTEGER,
  scoped_accuracy INTEGER,
  scoped_critical_hit_accuracy INTEGER,
  scoped_critical_hit_kills INTEGER,
  shots_fired INTEGER,
  shots_hit INTEGER,
  shots_missed INTEGER,
  scoped_shots_fired INTEGER,
  scoped_shots_hit INTEGER,
  weapon_accuracy INTEGER,
  hero_time_played FLOAT,

  UNIQUE (match_map_id, round, player_id, hero_id),
  FOREIGN KEY (match_map_id) REFERENCES match_maps(id) ON DELETE NO ACTION ON UPDATE NO ACTION,
  FOREIGN KEY (player_id) REFERENCES players(id) ON DELETE NO ACTION ON UPDATE NO ACTION,
  FOREIGN KEY (hero_id) REFERENCES heroes(id) ON DELETE NO ACTION ON UPDATE NO ACTION
);

DROP TABLE match_player_statistics;
ALTER TABLE match_player_statistics_new RENAME TO match_player_statistics;

-- match_events: переключаем с match_id на match_map_id
CREATE TABLE match_events_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  match_map_id INTEGER NOT NULL,
  time TIME,
  event TEXT,
  data JSON,
  FOREIGN KEY (match_map_id) REFERENCES match_maps(id) ON DELETE NO ACTION ON UPDATE NO ACTION
);

DROP TABLE match_events;
ALTER TABLE match_events_new RENAME TO match_events;

-- matches: убираем log_name
CREATE TABLE matches_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  tournament_id INTEGER,
  home_team_id INTEGER,
  away_team_id INTEGER,
  home_score INTEGER,
  away_score INTEGER,
  duration DATETIME_INTERVAL,

  created_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  modified_at DATETIME NOT NULL DEFAULT (CURRENT_TIMESTAMP),
  deleted_at DATETIME,

  FOREIGN KEY (home_team_id) REFERENCES teams(id) ON DELETE NO ACTION ON UPDATE NO ACTION,
  FOREIGN KEY (away_team_id) REFERENCES teams(id) ON DELETE NO ACTION ON UPDATE NO ACTION,
  FOREIGN KEY (tournament_id) REFERENCES tournaments(id) ON DELETE NO ACTION ON UPDATE NO ACTION
);

INSERT INTO matches_new SELECT id, tournament_id, home_team_id, away_team_id, home_score, away_score, duration, created_at, modified_at, deleted_at FROM matches;

DROP TABLE matches;
ALTER TABLE matches_new RENAME TO matches;
