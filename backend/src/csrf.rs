//! Origin checks for cookie-authenticated state-changing requests.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, Method, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{auth::SESSION_COOKIE, error::AppError, state::AppState};

fn origin_is_allowed(
    method: &Method,
    headers: &HeaderMap,
    frontend_url: &str,
    production: bool,
) -> bool {
    if !production
        || matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
        || headers.contains_key(header::AUTHORIZATION)
        || !has_session_cookie(headers)
    {
        return true;
    }

    headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        == Some(frontend_url)
}

fn has_session_cookie(headers: &HeaderMap) -> bool {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .any(|cookie| {
            cookie
                .trim()
                .split_once('=')
                .is_some_and(|(name, _)| name.trim() == SESSION_COOKIE)
        })
}

pub async fn layer(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if !origin_is_allowed(
        request.method(),
        request.headers(),
        &state.config.frontend_url,
        state.config.is_production(),
    ) {
        return AppError::Forbidden.into_response();
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, Method, header};

    use super::origin_is_allowed;

    const FRONTEND: &str = "https://gicreview.pages.dev";

    fn headers(cookie: Option<&str>, origin: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(cookie) = cookie {
            headers.insert(header::COOKIE, cookie.parse().unwrap());
        }
        if let Some(origin) = origin {
            headers.insert(header::ORIGIN, origin.parse().unwrap());
        }
        headers
    }

    #[test]
    fn production_cookie_writes_require_the_configured_origin() {
        let method = Method::POST;
        assert!(!origin_is_allowed(
            &method,
            &headers(Some("gic_session=token"), None),
            FRONTEND,
            true,
        ));
        assert!(origin_is_allowed(
            &method,
            &headers(Some("gic_session=token"), Some(FRONTEND)),
            FRONTEND,
            true,
        ));
        assert!(!origin_is_allowed(
            &method,
            &headers(Some("gic_session=token"), Some("https://attacker.example")),
            FRONTEND,
            true,
        ));
    }

    #[test]
    fn safe_and_non_cookie_requests_do_not_need_an_origin() {
        assert!(origin_is_allowed(
            &Method::GET,
            &headers(Some("gic_session=token"), None),
            FRONTEND,
            true,
        ));
        assert!(origin_is_allowed(
            &Method::POST,
            &headers(None, None),
            FRONTEND,
            true,
        ));
        assert!(origin_is_allowed(
            &Method::POST,
            &headers(Some("gic_session=token"), None),
            FRONTEND,
            false,
        ));
    }

    #[test]
    fn bearer_auth_requests_do_not_depend_on_browser_origin() {
        let mut headers = headers(Some("gic_session=token"), None);
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer gic_key"),
        );
        assert!(origin_is_allowed(&Method::POST, &headers, FRONTEND, true,));
    }
}
