use std::collections::HashMap;

use reqwest::header::HeaderMap;

type ReqwestHttpTransport = rmcp::transport::StreamableHttpClientTransport<reqwest::Client>;

/// Builds a `HeaderMap` from a `HashMap<String, String>` of user-provided headers.
///
/// Invalid header names or values are skipped.
fn build_header_map(headers: &HashMap<String, String>) -> HeaderMap {
    headers.try_into().unwrap_or_default()
}

/// Builds a reqwest client with custom headers for MCP HTTP/SSE connections.
///
/// Automatic redirects are disabled: the headers are installed as `default_headers`, so they
/// ride on every request this client issues, and reqwest only strips a fixed set of
/// credential headers (`Authorization`, `Cookie`, `Cookie2`, `Proxy-Authorization`,
/// `WWW-Authenticate`) when following a cross-host redirect. A user-configured `X-API-Key`
/// would otherwise be replayed verbatim to whatever host the MCP server points us at.
#[allow(clippy::result_large_err)]
pub fn build_client_with_headers(
    headers: &HashMap<String, String>,
) -> Result<reqwest::Client, rmcp::RmcpError> {
    let header_map = build_header_map(headers);

    reqwest::Client::builder()
        .default_headers(header_map)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| {
            rmcp::RmcpError::transport_creation::<ReqwestHttpTransport>(format!(
                "Failed to build client with headers: {e}",
            ))
        })
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::Arc;

    use axum::Router;
    use axum::extract::State;
    use axum::http::HeaderMap;
    use axum::http::StatusCode;
    use axum::http::header::LOCATION;
    use axum::response::IntoResponse;
    use axum::routing::post;
    use tokio::sync::Mutex;

    use super::*;

    const API_KEY_HEADER: &str = "x-api-key";
    const API_KEY_VALUE: &str = "secret";

    type CapturedHeader = Arc<Mutex<Option<String>>>;

    #[derive(Clone)]
    struct RedirectState {
        location: String,
        captured_header: CapturedHeader,
    }

    async fn capture_api_key_header(headers: &HeaderMap, captured_header: &CapturedHeader) {
        if let Some(value) = headers
            .get(API_KEY_HEADER)
            .and_then(|value| value.to_str().ok())
        {
            *captured_header.lock().await = Some(value.to_owned());
        }
    }

    async fn redirect_handler(
        State(state): State<RedirectState>,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        capture_api_key_header(&headers, &state.captured_header).await;

        (
            StatusCode::TEMPORARY_REDIRECT,
            [(LOCATION, state.location)],
            "",
        )
    }

    async fn redirected_handler(
        State(captured_header): State<CapturedHeader>,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        capture_api_key_header(&headers, &captured_header).await;

        (StatusCode::OK, "")
    }

    /// A malicious (or merely misconfigured) MCP endpoint must not be able to bounce our
    /// user-configured headers to a host of its choosing.
    #[tokio::test]
    async fn custom_headers_do_not_leak_to_redirect_target() -> anyhow::Result<()> {
        let redirected_header: CapturedHeader = Arc::new(Mutex::new(None));
        let redirected_listener =
            tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
        let redirected_addr = redirected_listener.local_addr()?;
        let _redirected_server = tokio::spawn({
            let redirected_header = redirected_header.clone();
            async move {
                let app = Router::new()
                    .route("/capture", post(redirected_handler))
                    .with_state(redirected_header);
                axum::serve(redirected_listener, app).await
            }
        });

        let original_header: CapturedHeader = Arc::new(Mutex::new(None));
        let redirect_listener =
            tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
        let redirect_addr = redirect_listener.local_addr()?;
        let _redirect_server = tokio::spawn({
            let state = RedirectState {
                location: format!("http://{redirected_addr}/capture"),
                captured_header: original_header.clone(),
            };
            async move {
                let app = Router::new()
                    .route("/mcp", post(redirect_handler))
                    .with_state(state);
                axum::serve(redirect_listener, app).await
            }
        });

        let headers = HashMap::from([(API_KEY_HEADER.to_owned(), API_KEY_VALUE.to_owned())]);
        let client = build_client_with_headers(&headers).expect("client builds");

        let response = client
            .post(format!("http://{redirect_addr}/mcp"))
            .send()
            .await?;

        // The header reached the endpoint the user configured...
        assert_eq!(
            original_header.lock().await.as_deref(),
            Some(API_KEY_VALUE),
            "custom header should be sent to the configured endpoint"
        );
        // ...and nowhere else.
        assert_eq!(
            redirected_header.lock().await.as_deref(),
            None,
            "custom header must not be replayed to the redirect target"
        );
        // The redirect is surfaced rather than followed.
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);

        Ok(())
    }
}
