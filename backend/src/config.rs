//! Configuration loaded from `backend/.env` (via `dotenvy`) and the process environment.
//!
//! The backend fails fast at startup with a clear message when a required variable
//! is missing or invalid. Secret values are never logged.

use std::{env, fmt};

use sqlx::mysql::{MySqlConnectOptions, MySqlSslMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppEnv {
    Development,
    Production,
}

#[derive(Clone)]
pub struct Config {
    pub app_env: AppEnv,
    pub backend_host: String,
    pub backend_port: u16,
    pub frontend_url: String,

    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
    pub db_user: String,
    pub db_password: String,
    pub db_ssl_required: bool,

    pub jwt_secret: String,
    pub jwt_expires_in_hours: i64,
    pub admin_email: String,
    pub admin_password: String,

    pub r2_access_key_id: String,
    pub r2_secret_access_key: String,
    pub r2_bucket: String,
    pub r2_endpoint: String,
    pub r2_public_base_url: String,

    pub max_image_size_bytes: usize,
    pub max_review_html_size_bytes: usize,
    /// Image uploads processed at once across all clients; the rest wait in a queue.
    pub max_concurrent_image_uploads: usize,

    /// In-memory response cache size in bytes (0 = disabled).
    pub cache_max_bytes: u64,
    pub cache_ttl_seconds: u64,
}

// Hand-written Debug so secrets can never end up in logs by accident.
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("app_env", &self.app_env)
            .field("backend_host", &self.backend_host)
            .field("backend_port", &self.backend_port)
            .field("frontend_url", &self.frontend_url)
            .field("db_host", &self.db_host)
            .field("db_name", &self.db_name)
            .field("db_ssl_required", &self.db_ssl_required)
            .field("r2_bucket", &self.r2_bucket)
            .field("r2_public_base_url", &self.r2_public_base_url)
            .field(
                "max_concurrent_image_uploads",
                &self.max_concurrent_image_uploads,
            )
            .finish_non_exhaustive()
    }
}

impl Config {
    pub fn is_production(&self) -> bool {
        self.app_env == AppEnv::Production
    }

    /// Use secure cross-site cookies when either production mode or the public frontend is HTTPS.
    pub fn uses_secure_cookies(&self) -> bool {
        self.is_production() || self.frontend_url.starts_with("https://")
    }

    /// MySQL connection options built from the `DB_*` values.
    pub fn connect_options(&self) -> MySqlConnectOptions {
        mysql_options(
            &self.db_host,
            self.db_port,
            &self.db_user,
            &self.db_password,
            &self.db_name,
            self.db_ssl_required,
        )
    }

    /// Load from the environment. Collects every problem before failing, so the operator
    /// can fix the whole `.env` in one go.
    pub fn from_env() -> Result<Self, String> {
        let mut errors: Vec<String> = Vec::new();

        let app_env = match optional("APP_ENV").as_deref() {
            None | Some("development") => AppEnv::Development,
            Some("production") => AppEnv::Production,
            Some(other) => {
                errors.push(format!(
                    "APP_ENV must be 'development' or 'production' (got '{other}')"
                ));
                AppEnv::Development
            }
        };

        let mut req = |name: &str| -> String {
            match optional(name) {
                Some(v) => v,
                None => {
                    errors.push(format!("{name} is required but missing or empty"));
                    String::new()
                }
            }
        };

        let backend_host = req("BACKEND_HOST");
        // Hosting platforms (Render, Railway, …) inject PORT; it wins over BACKEND_PORT.
        let backend_port_raw = match optional("PORT") {
            Some(port) => port,
            None => req("BACKEND_PORT"),
        };
        let frontend_url = req("FRONTEND_URL");
        let db_host = req("DB_HOST");
        let db_port_raw = optional("DB_PORT").unwrap_or_else(|| "3306".into());
        let db_name = req("DB_NAME");
        let db_user = req("DB_USER");
        let db_password = req("DB_PASSWORD");
        let jwt_secret = req("JWT_SECRET");
        let jwt_hours_raw = optional("JWT_EXPIRES_IN_HOURS").unwrap_or_else(|| "12".into());
        let admin_email = req("ADMIN_EMAIL");
        let admin_password = req("ADMIN_PASSWORD");
        let r2_access_key_id = req("R2_ACCESS_KEY_ID");
        let r2_secret_access_key = req("R2_SECRET_ACCESS_KEY");
        let r2_bucket = req("R2_BUCKET");
        let r2_endpoint_raw = optional("R2_ENDPOINT");
        let r2_account_id = optional("R2_ACCOUNT_ID");
        let r2_public_base_url = req("R2_PUBLIC_BASE_URL");
        let max_img_raw = optional("MAX_IMAGE_SIZE_MB").unwrap_or_else(|| "5".into());
        let max_html_raw = optional("MAX_REVIEW_HTML_SIZE_MB").unwrap_or_else(|| "10".into());
        let max_uploads_raw =
            optional("MAX_CONCURRENT_IMAGE_UPLOADS").unwrap_or_else(|| "5".into());
        let cache_mb_raw = optional("CACHE_MAX_MB").unwrap_or_else(|| "32".into());
        let cache_ttl_raw = optional("CACHE_TTL_SECONDS").unwrap_or_else(|| "300".into());

        let db_ssl_required = db_ssl_required_from_env().unwrap_or_else(|e| {
            errors.push(e);
            false
        });
        let backend_port = parse_num::<u16>("PORT/BACKEND_PORT", &backend_port_raw, &mut errors);
        let db_port = parse_num::<u16>("DB_PORT", &db_port_raw, &mut errors);
        let jwt_expires_in_hours =
            parse_num::<i64>("JWT_EXPIRES_IN_HOURS", &jwt_hours_raw, &mut errors);
        let max_img_mb = parse_num::<usize>("MAX_IMAGE_SIZE_MB", &max_img_raw, &mut errors);
        let max_html_mb = parse_num::<usize>("MAX_REVIEW_HTML_SIZE_MB", &max_html_raw, &mut errors);
        let max_uploads = parse_num::<usize>(
            "MAX_CONCURRENT_IMAGE_UPLOADS",
            &max_uploads_raw,
            &mut errors,
        );
        if max_uploads == 0 {
            errors.push("MAX_CONCURRENT_IMAGE_UPLOADS must be at least 1".into());
        }
        let cache_max_mb = parse_num::<u64>("CACHE_MAX_MB", &cache_mb_raw, &mut errors);
        let cache_ttl = parse_num::<u64>("CACHE_TTL_SECONDS", &cache_ttl_raw, &mut errors);
        if cache_max_mb > 0 && cache_ttl == 0 {
            errors.push("CACHE_TTL_SECONDS must be > 0 when the cache is enabled".into());
        }

        if !jwt_secret.is_empty() && jwt_secret.len() < 32 {
            errors.push("JWT_SECRET must be at least 32 characters long".into());
        }
        if !admin_password.is_empty() && admin_password.chars().count() < 8 {
            errors.push("ADMIN_PASSWORD must be at least 8 characters long".into());
        }
        if jwt_expires_in_hours <= 0 && !jwt_hours_raw.is_empty() {
            errors.push("JWT_EXPIRES_IN_HOURS must be > 0".into());
        }

        // R2_ENDPOINT may contain the literal placeholder `<R2_ACCOUNT_ID>` from the template.
        let r2_endpoint = match (r2_endpoint_raw, r2_account_id) {
            (Some(ep), Some(acc)) => ep.replace("<R2_ACCOUNT_ID>", &acc),
            (Some(ep), None) if !ep.contains('<') => ep,
            (None, Some(acc)) => format!("https://{acc}.r2.cloudflarestorage.com"),
            _ => {
                errors.push("R2_ENDPOINT (or R2_ACCOUNT_ID) is required".into());
                String::new()
            }
        };

        if !errors.is_empty() {
            return Err(format!(
                "invalid backend configuration (check backend/.env):\n  - {}",
                errors.join("\n  - ")
            ));
        }

        Ok(Self {
            app_env,
            backend_host,
            backend_port,
            frontend_url: frontend_url.trim_end_matches('/').to_string(),
            db_host,
            db_port,
            db_name,
            db_user,
            db_password,
            db_ssl_required,
            jwt_secret,
            jwt_expires_in_hours,
            admin_email: admin_email.to_lowercase(),
            admin_password,
            r2_access_key_id,
            r2_secret_access_key,
            r2_bucket,
            r2_endpoint,
            r2_public_base_url: r2_public_base_url.trim_end_matches('/').to_string(),
            max_image_size_bytes: max_img_mb * 1024 * 1024,
            max_review_html_size_bytes: max_html_mb * 1024 * 1024,
            max_concurrent_image_uploads: max_uploads,
            cache_max_bytes: cache_max_mb * 1024 * 1024,
            cache_ttl_seconds: cache_ttl,
        })
    }
}

/// `DB_SSL_REQUIRED=true|false` (default true).
/// - `true`: the MySQL connection must use SSL/TLS, otherwise connecting fails.
/// - `false`: SSL is not required (it is still used when the server offers it).
pub fn db_ssl_required_from_env() -> Result<bool, String> {
    match optional("DB_SSL_REQUIRED").map(|v| v.to_ascii_lowercase()) {
        None => Ok(true),
        Some(v) if matches!(v.as_str(), "true" | "1" | "yes") => Ok(true),
        Some(v) if matches!(v.as_str(), "false" | "0" | "no") => Ok(false),
        Some(v) => Err(format!("DB_SSL_REQUIRED must be true or false (got '{v}')")),
    }
}

fn mysql_options(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    db: &str,
    ssl_required: bool,
) -> MySqlConnectOptions {
    MySqlConnectOptions::new()
        .host(host)
        .port(port)
        .username(user)
        .password(password)
        .database(db)
        .charset("utf8mb4")
        .ssl_mode(if ssl_required {
            MySqlSslMode::Required
        } else {
            MySqlSslMode::Preferred
        })
}

/// Database connection options from `DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USER`, `DB_PASSWORD`
/// and the optional `DB_SSL_REQUIRED` only (no JWT/R2 needed). Used by `cargo run -- migrate` and by the integration tests, so a
/// separate `DATABASE_URL` is never required.
pub fn db_options_from_env() -> Result<MySqlConnectOptions, String> {
    let mut missing = Vec::new();
    let mut get = |name: &str| {
        optional(name).unwrap_or_else(|| {
            missing.push(name.to_string());
            String::new()
        })
    };
    let host = get("DB_HOST");
    let name = get("DB_NAME");
    let user = get("DB_USER");
    let password = get("DB_PASSWORD");
    if !missing.is_empty() {
        return Err(format!("missing in backend/.env: {}", missing.join(", ")));
    }
    let port = optional("DB_PORT")
        .map(|p| {
            p.parse::<u16>()
                .map_err(|_| format!("DB_PORT must be a number (got '{p}')"))
        })
        .transpose()?
        .unwrap_or(3306);
    let ssl_required = db_ssl_required_from_env()?;
    Ok(mysql_options(
        &host,
        port,
        &user,
        &password,
        &name,
        ssl_required,
    ))
}

/// Read a variable, treating empty strings as missing. Strips a trailing ` # comment`
/// in case the env loader kept inline comments.
fn optional(name: &str) -> Option<String> {
    let raw = env::var(name).ok()?;
    let value = match raw.find(" #") {
        Some(idx) => &raw[..idx],
        None => raw.as_str(),
    };
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn parse_num<T: std::str::FromStr + Default>(name: &str, raw: &str, errors: &mut Vec<String>) -> T {
    if raw.is_empty() {
        return T::default();
    }
    raw.parse::<T>().unwrap_or_else(|_| {
        errors.push(format!("{name} must be a number (got '{raw}')"));
        T::default()
    })
}
