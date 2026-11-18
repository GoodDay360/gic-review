-- Years group subjects (e.g. "I3", "I4", or any custom name). Students pick a year first,
-- then a subject of that year. Every subject belongs to exactly one year.
CREATE TABLE years (
    id          BIGINT       NOT NULL AUTO_INCREMENT PRIMARY KEY,
    slug        VARCHAR(100) NOT NULL,
    name        VARCHAR(100) NOT NULL,
    description TEXT         NULL,
    sort_order  INT          NOT NULL DEFAULT 0,
    created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY uq_years_slug (slug)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

-- Existing subjects go into a default year that admins can rename (e.g. to "I4").
INSERT INTO years (slug, name, description, sort_order)
VALUES ('general', 'General', 'Rename this year (e.g. I4) in the admin dashboard.', 0);

ALTER TABLE subjects ADD COLUMN year_id BIGINT NULL AFTER id;
UPDATE subjects SET year_id = (SELECT id FROM years WHERE slug = 'general');
ALTER TABLE subjects MODIFY year_id BIGINT NOT NULL;
ALTER TABLE subjects
    ADD KEY idx_subjects_year (year_id, sort_order),
    ADD CONSTRAINT fk_subjects_year FOREIGN KEY (year_id) REFERENCES years (id) ON DELETE RESTRICT;
