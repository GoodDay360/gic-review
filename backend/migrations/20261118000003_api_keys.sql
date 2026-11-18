-- API keys let a manager (or an AI working for them) call the content API with
-- `Authorization: Bearer gic_...`. Only a SHA-256 hash of the key is stored.
CREATE TABLE api_keys (
    id           BIGINT       NOT NULL AUTO_INCREMENT PRIMARY KEY,
    user_id      BIGINT       NOT NULL,
    name         VARCHAR(100) NOT NULL,
    key_prefix   VARCHAR(16)  NOT NULL, -- first characters, shown in the UI to recognise the key
    key_hash     CHAR(64)     NOT NULL, -- hex SHA-256 of the full key
    scopes       VARCHAR(255) NOT NULL, -- comma-separated, e.g. 'quizzes:write,reviews:write'
    expires_at   DATETIME     NULL,
    last_used_at DATETIME     NULL,
    revoked_at   DATETIME     NULL,
    created_at   DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE KEY uq_api_keys_hash (key_hash),
    KEY idx_api_keys_user (user_id),
    CONSTRAINT fk_api_keys_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;
