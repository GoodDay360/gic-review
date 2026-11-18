-- Share a whole year, or one subject of a year, with other users (by email, dashboard only).
-- Recipients can add content to the shared year's subjects / the shared subject, and view, edit
-- and publish all content in it. Only the year's owner (or an admin) manages the year, its
-- subjects and who it is shared with; recipients can leave.
CREATE TABLE year_shares (
    year_id    BIGINT   NOT NULL,
    user_id    BIGINT   NOT NULL,
    granted_by BIGINT   NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (year_id, user_id),
    KEY idx_year_shares_user (user_id),
    CONSTRAINT fk_year_shares_year FOREIGN KEY (year_id) REFERENCES years (id) ON DELETE CASCADE,
    CONSTRAINT fk_year_shares_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT fk_year_shares_granted_by FOREIGN KEY (granted_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE subject_shares (
    subject_id BIGINT   NOT NULL,
    user_id    BIGINT   NOT NULL,
    granted_by BIGINT   NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (subject_id, user_id),
    KEY idx_subject_shares_user (user_id),
    CONSTRAINT fk_subject_shares_subject FOREIGN KEY (subject_id) REFERENCES subjects (id) ON DELETE CASCADE,
    CONSTRAINT fk_subject_shares_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT fk_subject_shares_granted_by FOREIGN KEY (granted_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

-- Item-level sharing is replaced by year/subject sharing: sharing a subject (or its year) gives
-- access to every quiz and review in it.
DROP TABLE quiz_shares;
DROP TABLE review_shares;
