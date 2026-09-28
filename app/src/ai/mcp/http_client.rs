use std::collections::HashMap;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;
use std::time::Duration;

use reqwest::Url;
use reqwest::dns::Addrs;
use reqwest::dns::Name;
use reqwest::dns::Resolve;
use reqwest::dns::Resolving;
use reqwest::header::HeaderMap;

type ReqwestHttpTransport = rmcp::transport::StreamableHttpClientTransport<reqwest::Client>;

/// Matches the timeout rmcp's own default auth client uses, so injecting ours does not
/// silently change timeout behaviour.
const OAUTH_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(30);

/// Same bound upstream rmcp applies to its discovery redirect chain.
const MAX_OAUTH_DISCOVERY_REDIRECTS: usize = 5;

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

/// Builds the reqwest client used to drive rmcp's OAuth state machine.
///
/// rmcp's `extract_resource_metadata_url_from_header` accepts any absolute URL out of a
/// server-controlled `WWW-Authenticate: Bearer resource_metadata="..."` header and then GETs
/// it, with no origin or address checks. Discovery runs inside `start_authorization()` —
/// before the user ever sees a consent screen — so a hostile MCP server could use it to reach
/// cloud metadata endpoints, localhost services, or anything else on the user's network.
///
/// We cannot fix that inside our pinned rmcp rev, but `AuthorizationManager::new` performs no
/// I/O and `OAuthState::new`'s second argument replaces its HTTP client before any discovery
/// runs, so hardening the client covers the whole discovery path.
pub fn build_oauth_discovery_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(OAUTH_DISCOVERY_TIMEOUT)
        .redirect(same_origin_redirect_policy())
        .dns_resolver(std::sync::Arc::new(PublicOnlyResolver))
        .build()
}

/// Follows redirects only within a single origin, so a redirect cannot be used to reach a host
/// that was never vetted. Upstream rmcp's SSRF fix applies the same rule to discovery.
fn same_origin_redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() > MAX_OAUTH_DISCOVERY_REDIRECTS {
            return attempt.stop();
        }
        match attempt.previous().last() {
            Some(previous) if is_same_origin(previous, attempt.url()) => attempt.follow(),
            _ => attempt.stop(),
        }
    })
}

fn is_same_origin(base: &Url, candidate: &Url) -> bool {
    base.scheme() == candidate.scheme()
        && base
            .host_str()
            .zip(candidate.host_str())
            .is_some_and(|(base, candidate)| base.eq_ignore_ascii_case(candidate))
        && base.port_or_known_default() == candidate.port_or_known_default()
}

/// A DNS resolver that drops any address on the local machine or the local network.
///
/// Filtering at resolution time rather than at the URL, as upstream rmcp does, also covers
/// hostnames that resolve into private space — `localhost`, `metadata.google.internal`, or an
/// attacker-controlled name pointed at `169.254.169.254`.
struct PublicOnlyResolver;

impl Resolve for PublicOnlyResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            // Port 0 is a placeholder; reqwest substitutes the real port after resolution.
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await?
                .filter(|addr| !is_internal_ip(addr.ip()))
                .collect();

            if addrs.is_empty() {
                return Err(format!(
                    "refusing to connect to `{host}`: it resolves only to local or \
                     private-network addresses"
                )
                .into());
            }

            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

fn is_internal_ip(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(addr) => is_internal_ipv4(addr),
        IpAddr::V6(addr) => is_internal_ipv6(addr),
    }
}

fn is_internal_ipv4(addr: Ipv4Addr) -> bool {
    let octets = addr.octets();
    addr.is_private()
        || addr.is_loopback()
        || addr.is_link_local()
        || addr.is_broadcast()
        || addr.is_unspecified()
        || addr.is_multicast()
        // "This network" (0.0.0.0/8).
        || octets[0] == 0
        // Carrier-grade NAT (100.64.0.0/10).
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        // Benchmarking (198.18.0.0/15).
        || (octets[0] == 198 && matches!(octets[1], 18 | 19))
}

fn is_internal_ipv6(addr: Ipv6Addr) -> bool {
    if let Some(mapped) = addr.to_ipv4_mapped() {
        return is_internal_ipv4(mapped);
    }

    let segments = addr.segments();
    addr.is_loopback()
        || addr.is_unspecified()
        || addr.is_multicast()
        // Link-local unicast (fe80::/10).
        || (segments[0] & 0xffc0) == 0xfe80
        // Unique local addresses (fc00::/7).
        || (segments[0] & 0xfe00) == 0xfc00
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn oauth_discovery_resolver_rejects_local_and_private_addresses() {
        // Addresses a hostile MCP server could try to reach through OAuth discovery.
        for addr in [
            "127.0.0.1",
            "127.1.2.3",
            "0.0.0.0",
            "0.1.2.3",
            "10.0.0.1",
            "172.16.0.1",
            "172.31.255.254",
            "192.168.1.1",
            // Cloud instance metadata.
            "169.254.169.254",
            "255.255.255.255",
            "224.0.0.1",
            // Carrier-grade NAT and benchmarking ranges.
            "100.64.0.1",
            "100.127.255.255",
            "198.18.0.1",
            "198.19.255.255",
            "::1",
            "::",
            "fe80::1",
            "fc00::1",
            "fd12:3456::1",
            "ff02::1",
            // IPv4-mapped forms of the same private space.
            "::ffff:127.0.0.1",
            "::ffff:169.254.169.254",
        ] {
            let ip: IpAddr = addr.parse().expect("test address parses");
            assert!(is_internal_ip(ip), "{addr} should be rejected");
        }
    }

    #[test]
    fn oauth_discovery_resolver_allows_public_addresses() {
        for addr in [
            "1.1.1.1",
            "8.8.8.8",
            "140.82.121.4",
            // Adjacent to, but outside, the blocked ranges.
            "100.63.255.255",
            "100.128.0.1",
            "172.15.255.255",
            "172.32.0.1",
            "198.17.255.255",
            "198.20.0.1",
            "2606:4700:4700::1111",
            "2001:4860:4860::8888",
        ] {
            let ip: IpAddr = addr.parse().expect("test address parses");
            assert!(!is_internal_ip(ip), "{addr} should be allowed");
        }
    }

    #[tokio::test]
    async fn oauth_discovery_client_refuses_to_resolve_localhost() {
        let client = build_oauth_discovery_client().expect("client builds");

        let error = client
            .get("http://localhost/.well-known/oauth-protected-resource")
            .send()
            .await
            .expect_err("localhost must not be reachable");

        // Assert on the resolver's own message so this cannot pass on a plain connection
        // refusal from a host that happens to have nothing listening.
        let mut chain = String::new();
        let mut source: Option<&dyn std::error::Error> = Some(&error);
        while let Some(err) = source {
            chain.push_str(&err.to_string());
            chain.push('\n');
            source = err.source();
        }
        assert!(
            chain.contains("refusing to connect"),
            "expected the resolver to reject localhost, got: {chain}"
        );
    }

    #[test]
    fn same_origin_compares_scheme_host_and_effective_port() {
        let base = Url::parse("https://mcp.example.com/a").unwrap();

        for same in [
            "https://mcp.example.com/b",
            "https://MCP.example.com/b",
            "https://mcp.example.com:443/b",
        ] {
            assert!(
                is_same_origin(&base, &Url::parse(same).unwrap()),
                "{same} should be same-origin"
            );
        }

        for different in [
            "http://mcp.example.com/b",
            "https://evil.example.com/b",
            "https://mcp.example.com:8443/b",
            "https://mcp.example.com.evil.com/b",
        ] {
            assert!(
                !is_same_origin(&base, &Url::parse(different).unwrap()),
                "{different} should not be same-origin"
            );
        }
    }
}
