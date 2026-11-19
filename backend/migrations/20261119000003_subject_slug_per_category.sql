-- Subject slugs only need to be unique inside their category, so two categories (e.g. "I3" and
-- "I4") can each have an "hci" subject. Public URLs carry both slugs: /categories/i4/hci/...
ALTER TABLE subjects
    DROP INDEX uq_subjects_slug,
    ADD UNIQUE KEY uq_subjects_category_slug (category_id, slug);
