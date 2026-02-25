-- Role this player fills in the tournament (tank | damage | support | flex)
ALTER TABLE players ADD COLUMN role TEXT;

-- Division/tier this player belongs to within the tournament
ALTER TABLE players ADD COLUMN division_id INTEGER
  REFERENCES divisions(id) ON DELETE NO ACTION ON UPDATE NO ACTION;

-- Link to the player's tournament registration (nullable: some players may have no registration)
ALTER TABLE players ADD COLUMN registration_id INTEGER
  REFERENCES registrations(id) ON DELETE NO ACTION ON UPDATE NO ACTION;

-- Substitution tracking
-- NULL replaces_player_id = original roster member
ALTER TABLE players ADD COLUMN replaces_player_id INTEGER
  REFERENCES players(id) ON DELETE NO ACTION ON UPDATE NO ACTION;

-- Match range during which this player is on the active roster
-- NULL active_from_match_id = active from the first match of the tournament
-- NULL active_to_match_id   = active until the last match of the tournament
ALTER TABLE players ADD COLUMN active_from_match_id INTEGER
  REFERENCES matches(id) ON DELETE NO ACTION ON UPDATE NO ACTION;
ALTER TABLE players ADD COLUMN active_to_match_id INTEGER
  REFERENCES matches(id) ON DELETE NO ACTION ON UPDATE NO ACTION;
