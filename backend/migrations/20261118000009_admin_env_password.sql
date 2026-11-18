-- Argon2 hash of the ADMIN_PASSWORD (from backend/.env) last applied to this admin account.
-- On startup the server compares it with the current ADMIN_PASSWORD: if the value in .env
-- changed, the admin's password is updated; if not, a password changed in the dashboard is kept.
ALTER TABLE users ADD COLUMN env_password_hash VARCHAR(255) NULL AFTER session_version;
