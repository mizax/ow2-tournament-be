-- checkin.from/to were previously stored as "HH:MM" time-only strings.
-- They must now be ISO 8601 datetimes (matching DateTime<Utc> serde format).
-- Remove any values that are 5 characters long (i.e. "HH:MM") since they
-- are not valid datetimes and would cause JSON deserialization errors.

UPDATE tournament_configuration
SET configuration_json = json_remove(configuration_json, '$.registration.checkin.from')
WHERE length(json_extract(configuration_json, '$.registration.checkin.from')) = 5;

UPDATE tournament_configuration
SET configuration_json = json_remove(configuration_json, '$.registration.checkin.to')
WHERE length(json_extract(configuration_json, '$.registration.checkin.to')) = 5;
