INSERT INTO heroes (name) VALUES
  ('Domina'),
  ('Anran'),
  ('Emre'),
  ('Mizuki'),
  ('Jetpack Cat');

INSERT INTO heroes_localization (hero_id, name, lang) VALUES
  ((SELECT id FROM heroes WHERE name='Domina'),'Domina','en'),
  ((SELECT id FROM heroes WHERE name='Anran'),'Anran','en'),
  ((SELECT id FROM heroes WHERE name='Emre'),'Emre','en'),
  ((SELECT id FROM heroes WHERE name='Mizuki'),'Mizuki','en'),
  ((SELECT id FROM heroes WHERE name='Jetpack Cat'),'Jetpack Cat','en');

INSERT INTO heroes_localization (hero_id, name, lang) VALUES
  ((SELECT id FROM heroes WHERE name='Domina'),'Домина','ru'),
  ((SELECT id FROM heroes WHERE name='Anran'),'Ань Жань','ru'),
  ((SELECT id FROM heroes WHERE name='Emre'),'Эмре','ru'),
  ((SELECT id FROM heroes WHERE name='Mizuki'),'Мидзуки','ru'),
  ((SELECT id FROM heroes WHERE name='Jetpack Cat'),'Реактивная Киса','ru');
