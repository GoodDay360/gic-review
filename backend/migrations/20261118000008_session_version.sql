-- Bumped on every password change (own change or admin reset). Session tokens carry the
-- version they were issued with; older versions are rejected, so a password change signs out
-- every other device.
ALTER TABLE users ADD COLUMN session_version INT NOT NULL DEFAULT 0 AFTER password_hash;
