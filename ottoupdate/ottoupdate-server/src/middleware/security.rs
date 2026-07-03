use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

#[derive(Debug, Clone, Default)]
pub struct SecurityConfig {
    pub bearer_token: Option<String>,
}

pub async fn security_middleware(
    State(config): State<SecurityConfig>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();

    if path != "/health" && path.starts_with("/v1") {
        if let Some(expected) = config.bearer_token.as_deref() {
            let header = request
                .headers()
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            let received = header.strip_prefix("Bearer ").unwrap_or_default();
            if received != expected {
                return (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(serde_json::json!({ "error": "unauthorized" })),
                )
                    .into_response();
            }
        }
    }

    let mut response = next.run(request).await;
    response.headers_mut().insert(
        "X-Content-Type-Options",
        "nosniff".parse().expect("static header value"),
    );
    response.headers_mut().insert(
        "X-Frame-Options",
        "DENY".parse().expect("static header value"),
    );
    response.headers_mut().insert(
        "Referrer-Policy",
        "no-referrer".parse().expect("static header value"),
    );
    response
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::middleware::from_fn_with_state;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    use super::{security_middleware, SecurityConfig};

    #[tokio::test]
    async fn requires_bearer_token_when_configured() {
        let app = Router::new()
            .route("/v1/state", get(|| async { "ok" }))
            .layer(from_fn_with_state(
                SecurityConfig {
                    bearer_token: Some("token-1".to_string()),
                },
                security_middleware,
            ));

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/state")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("response expected");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
