-- "Years" are renamed to "categories": subjects are grouped by category (e.g. "I3", "I4",
-- or any custom name). Same data and rules, only the names change.
RENAME TABLE years TO categories, year_shares TO category_shares;

-- categories
ALTER TABLE categories DROP FOREIGN KEY fk_years_created_by;
ALTER TABLE categories
    RENAME INDEX uq_years_slug TO uq_categories_slug,
    RENAME INDEX fk_years_created_by TO fk_categories_created_by;
ALTER TABLE categories
    ADD CONSTRAINT fk_categories_created_by FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL;

UPDATE categories
SET description = 'Rename this category (e.g. I4) in the admin dashboard.'
WHERE slug = 'general' AND description = 'Rename this year (e.g. I4) in the admin dashboard.';

-- subjects.year_id -> subjects.category_id
ALTER TABLE subjects DROP FOREIGN KEY fk_subjects_year;
ALTER TABLE subjects
    RENAME COLUMN year_id TO category_id,
    RENAME INDEX idx_subjects_year TO idx_subjects_category;
ALTER TABLE subjects
    ADD CONSTRAINT fk_subjects_category FOREIGN KEY (category_id) REFERENCES categories (id) ON DELETE RESTRICT;

-- category_shares.year_id -> category_shares.category_id
ALTER TABLE category_shares
    DROP FOREIGN KEY fk_year_shares_year,
    DROP FOREIGN KEY fk_year_shares_user,
    DROP FOREIGN KEY fk_year_shares_granted_by;
ALTER TABLE category_shares
    RENAME COLUMN year_id TO category_id,
    RENAME INDEX idx_year_shares_user TO idx_category_shares_user,
    RENAME INDEX fk_year_shares_granted_by TO fk_category_shares_granted_by;
ALTER TABLE category_shares
    ADD CONSTRAINT fk_category_shares_category FOREIGN KEY (category_id) REFERENCES categories (id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_category_shares_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    ADD CONSTRAINT fk_category_shares_granted_by FOREIGN KEY (granted_by) REFERENCES users (id) ON DELETE SET NULL;
