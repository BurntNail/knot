ALTER TABLE people
ADD COLUMN person_name TEXT NOT NULL;

ALTER TABLE people
DROP COLUMN IF EXISTS first_name;

ALTER TABLE people
DROP COLUMN IF EXISTS surname;

ALTER TABLE people
DROP COLUMN IF EXISTS form;