-- Years belong to the user who created them. Managers create their own years (and subjects
-- in them) in the dashboard and can only add quizzes/reviews to subjects of years they own.
-- Admins manage every year. NULL = no owner (e.g. the seeded "General" year): admins only.
ALTER TABLE years
    ADD COLUMN created_by BIGINT NULL AFTER sort_order,
    ADD CONSTRAINT fk_years_created_by FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL;
