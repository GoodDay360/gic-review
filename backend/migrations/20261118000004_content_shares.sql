-- Content ownership and sharing between managers.
-- The owner of a quiz/review is its `created_by`. The owner can share it with other managers
-- (by email); shared managers can edit and publish it, but only the owner can delete it or
-- change who it is shared with.
CREATE TABLE quiz_shares (
    quiz_id    BIGINT   NOT NULL,
    user_id    BIGINT   NOT NULL,
    granted_by BIGINT   NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (quiz_id, user_id),
    KEY idx_quiz_shares_user (user_id),
    CONSTRAINT fk_quiz_shares_quiz FOREIGN KEY (quiz_id) REFERENCES quizzes (id) ON DELETE CASCADE,
    CONSTRAINT fk_quiz_shares_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT fk_quiz_shares_granted_by FOREIGN KEY (granted_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE review_shares (
    review_id  BIGINT   NOT NULL,
    user_id    BIGINT   NOT NULL,
    granted_by BIGINT   NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (review_id, user_id),
    KEY idx_review_shares_user (user_id),
    CONSTRAINT fk_review_shares_review FOREIGN KEY (review_id) REFERENCES interactive_reviews (id) ON DELETE CASCADE,
    CONSTRAINT fk_review_shares_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    CONSTRAINT fk_review_shares_granted_by FOREIGN KEY (granted_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;
