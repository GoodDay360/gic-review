-- Initial schema for GIC Review.
-- Never edit this file after it has been applied anywhere: add a new migration instead.

CREATE TABLE users (
    id            BIGINT       NOT NULL AUTO_INCREMENT PRIMARY KEY,
    email         VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    display_name  VARCHAR(100) NOT NULL,
    role          ENUM('admin', 'manager') NOT NULL DEFAULT 'manager',
    is_active     BOOLEAN      NOT NULL DEFAULT TRUE,
    created_by    BIGINT       NULL,
    created_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY uq_users_email (email),
    CONSTRAINT fk_users_created_by FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE subjects (
    id          BIGINT       NOT NULL AUTO_INCREMENT PRIMARY KEY,
    slug        VARCHAR(100) NOT NULL,
    name        VARCHAR(150) NOT NULL,
    description TEXT         NULL,
    color       VARCHAR(20)  NOT NULL DEFAULT 'mauve', -- a Catppuccin accent name
    sort_order  INT          NOT NULL DEFAULT 0,
    created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY uq_subjects_slug (slug)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE interactive_reviews (
    id           BIGINT       NOT NULL AUTO_INCREMENT PRIMARY KEY,
    subject_id   BIGINT       NOT NULL,
    title        VARCHAR(200) NOT NULL,
    slug         VARCHAR(100) NOT NULL,
    r2_key       VARCHAR(255) NOT NULL,
    sort_order   INT          NOT NULL DEFAULT 0,
    is_published BOOLEAN      NOT NULL DEFAULT FALSE,
    created_by   BIGINT       NULL,
    created_at   DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY uq_reviews_subject_slug (subject_id, slug),
    CONSTRAINT fk_reviews_subject FOREIGN KEY (subject_id) REFERENCES subjects (id) ON DELETE RESTRICT,
    CONSTRAINT fk_reviews_created_by FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE quizzes (
    id              BIGINT       NOT NULL AUTO_INCREMENT PRIMARY KEY,
    subject_id      BIGINT       NOT NULL,
    title           VARCHAR(200) NOT NULL,
    description     TEXT         NULL,
    cover_image_key VARCHAR(255) NULL, -- R2 key, shown on the quiz card
    is_published    BOOLEAN      NOT NULL DEFAULT FALSE,
    created_by      BIGINT       NULL,
    created_at      DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    KEY idx_quizzes_subject (subject_id),
    CONSTRAINT fk_quizzes_subject FOREIGN KEY (subject_id) REFERENCES subjects (id) ON DELETE RESTRICT,
    CONSTRAINT fk_quizzes_created_by FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE questions (
    id                    BIGINT       NOT NULL AUTO_INCREMENT PRIMARY KEY,
    quiz_id               BIGINT       NOT NULL,
    position              INT          NOT NULL DEFAULT 0,
    prompt                TEXT         NULL, -- text and/or image required
    image_key             VARCHAR(255) NULL, -- question image / flashcard front
    image_alt             VARCHAR(255) NULL,
    answer_text           TEXT         NULL, -- flashcard back / short answer
    answer_image_key      VARCHAR(255) NULL, -- flashcard back image
    answer_image_alt      VARCHAR(255) NULL,
    explanation           TEXT         NULL,
    explanation_image_key VARCHAR(255) NULL, -- e.g. a diagram shown after answering
    multi_select          BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at            DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at            DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    KEY idx_questions_quiz (quiz_id, position),
    CONSTRAINT fk_questions_quiz FOREIGN KEY (quiz_id) REFERENCES quizzes (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE question_options (
    id          BIGINT       NOT NULL AUTO_INCREMENT PRIMARY KEY,
    question_id BIGINT       NOT NULL,
    position    INT          NOT NULL DEFAULT 0,
    text        TEXT         NULL, -- text and/or image required
    image_key   VARCHAR(255) NULL, -- image answer choice
    image_alt   VARCHAR(255) NULL,
    is_correct  BOOLEAN      NOT NULL DEFAULT FALSE,
    KEY idx_options_question (question_id, position),
    CONSTRAINT fk_options_question FOREIGN KEY (question_id) REFERENCES questions (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;
