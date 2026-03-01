ALTER TABLE tournaments ADD COLUMN status TEXT NOT NULL DEFAULT 'draft';

-- For prod: migrate status from JSON config
UPDATE tournaments
SET status = COALESCE(
    json_extract(
        (SELECT configuration_json FROM tournament_configuration WHERE tournament_id = tournaments.id),
        '$.status'
    ),
    'draft'
);

-- For prod: remove status from JSON config (SQLite 3.38+)
UPDATE tournament_configuration
SET configuration_json = json_remove(configuration_json, '$.status');
