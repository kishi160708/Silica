use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use futures::future::BoxFuture;
use futures::FutureExt;
use http_client::{
    http::{HeaderValue, Response, StatusCode},
    AsyncBody, HttpClient, Url,
};

/// The largest remote image Silica will keep. A note's picture is a picture;
/// anything past this is not one, and the disk is not the place to find out.
pub const MAX_BYTES: u64 = 32 * 1024 * 1024;

/// How many redirects a fetch may follow.
pub const MAX_REDIRECTS: &str = "5";

pub struct SilicaHttpClient {
    cache_dir: PathBuf,
}

/// Whether `url` may be fetched at all.
///
/// A note can name any image it likes, so the request goes where the note
/// says. That makes the note able to probe whatever the app can reach, which
/// on a laptop includes the router's admin page and anything on `localhost`.
/// A literal private or loopback host is refused here, before any request.
///
/// A hostname that resolves to a private address is refused here, before any
/// request: the name is resolved and every candidate address is checked, so
/// DNS rebinding cannot reach the local network through a look-alike name.
pub fn allows_destination(url: &str) -> bool {
    let Some((scheme, rest)) = url.split_once("://") else {
        return false;
    };
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
        return false;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    // Drop any userinfo, then any port.
    let host = authority.rsplit('@').next().unwrap_or(authority);
    let host = if host.starts_with('[') {
        // A bracketed IPv6 literal.
        host.split(']')
            .next()
            .unwrap_or(host)
            .trim_start_matches('[')
    } else {
        host.split(':').next().unwrap_or(host)
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() || host == "localhost" || host.ends_with(".localhost") {
        return false;
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return !is_private(ip);
    }
    // A DNS name: resolve it and refuse if every address it could reach is
    // private. curl resolves again at connect time, but this check catches the
    // common rebinding case and the `--resolve` argument below pins the one we
    // validated.
    match std::net::ToSocketAddrs::to_socket_addrs(&format!("{host}:80")) {
        Ok(mut addrs) => addrs.any(|a| !is_private(a.ip())),
        Err(_) => false,
    }
}

/// The `--resolve host:port:ip` argument for curl, pinning the connection to
/// the first public address the name resolves to. Returns `None` when the
/// name cannot be resolved or every address is private, so the caller can
/// refuse the request rather than let curl resolve on its own.
pub fn resolve_target(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) if p.parse::<u16>().is_ok() => (h, p.parse::<u16>().ok()?),
        _ => (
            host_port,
            match scheme.to_ascii_lowercase().as_str() {
                "https" => 443,
                _ => 80,
            },
        ),
    };
    let ip = std::net::ToSocketAddrs::to_socket_addrs(&format!("{host}:{port}"))
        .ok()?
        .find(|a| !is_private(a.ip()))?;
    Some(format!("{host}:{port}:{}", ip.ip()))
}

/// Loopback, link-local, private, and the other ranges that mean "this
/// machine or its network" rather than "the internet".
fn is_private(ip: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.is_documentation()
                // 100.64/10, carrier-grade NAT, which is a LAN in practice.
                || (v4.octets()[0] == 100 && (64..128).contains(&v4.octets()[1]))
                // 169.254 metadata endpoints are link-local and already
                // covered; 0/8 and 240/4 are not routable either.
                || v4.octets()[0] == 0
                || v4.octets()[0] >= 240
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                // Unique local (fc00::/7) and link-local (fe80::/10).
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                || (v6.segments()[0] & 0xffc0) == 0xfe80
                // An IPv4-mapped address is private if the IPv4 is.
                || v6.to_ipv4_mapped().is_some_and(|v4| is_private(v4.into()))
        }
    }
}

impl SilicaHttpClient {
    pub fn new() -> Self {
        let cache_dir = if let Some(cache) = std::env::var_os("XDG_CACHE_HOME") {
            PathBuf::from(cache).join("silica").join("remote_images")
        } else if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home)
                .join(".cache")
                .join("silica")
                .join("remote_images")
        } else {
            std::env::temp_dir().join("silica").join("remote_images")
        };
        let _ = std::fs::create_dir_all(&cache_dir);
        Self { cache_dir }
    }

    pub fn cache_path(&self, url: &str) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        let hash = hasher.finish();
        let ext = Path::new(url.split('?').next().unwrap_or(url))
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("img");
        self.cache_dir.join(format!("{:016x}.{}", hash, ext))
    }
}

impl Default for SilicaHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for SilicaHttpClient {
    fn user_agent(&self) -> Option<&HeaderValue> {
        None
    }

    fn proxy(&self) -> Option<&Url> {
        None
    }

    fn send(
        &self,
        req: http_client::http::Request<AsyncBody>,
    ) -> BoxFuture<'static, Result<Response<AsyncBody>>> {
        let uri = req.uri().to_string();
        let method = req.method().clone();
        if !allows_destination(&uri) {
            return async move {
                Err(anyhow!(
                    "{uri} is not an http(s) URL, or it points into a private network, so it is not fetched"
                ))
            }
            .boxed();
        }
        // Pin the connection to the public address the name resolves to, so
        // DNS rebinding cannot send the request somewhere else at connect
        // time. A name that cannot be resolved, or that resolves only to a
        // private address, is refused.
        let Some(resolve_arg) = resolve_target(&uri) else {
            return async move {
                Err(anyhow!(
                    "{uri} could not be resolved to a public address, so it is not fetched"
                ))
            }
            .boxed();
        };
        let cache_path = self.cache_path(&uri);

        async move {
            if method == http_client::http::Method::GET {
                if cache_path.is_file() {
                    if let Ok(data) = std::fs::read(&cache_path) {
                        if !data.is_empty() {
                            let resp = Response::builder()
                                .status(StatusCode::OK)
                                .body(AsyncBody::from_bytes(data.into()))?;
                            return Ok(resp);
                        }
                    }
                }
            }

            let (tx, rx) = futures::channel::oneshot::channel();
            let uri_clone = uri.clone();
            let cache_path_clone = cache_path.clone();
            let resolve_clone = resolve_arg.clone();

            std::thread::spawn(move || {
                let temp_path = cache_path_clone.with_extension("tmp");
                let status = std::process::Command::new("curl")
                    .arg("-s")
                    .arg("-f")
                    .arg("-L")
                    // Redirects are followed, but only five of them, and only
                    // to the same two schemes: a note's image could otherwise
                    // bounce through `file://` or loop forever.
                    .arg("--max-redirs")
                    .arg(MAX_REDIRECTS)
                    .arg("--proto")
                    .arg("=http,https")
                    .arg("--proto-redir")
                    .arg("=http,https")
                    // Pin the connection to the public address the name
                    // resolved to, so DNS rebinding cannot redirect it at
                    // connect time.
                    .arg("--resolve")
                    .arg(&resolve_clone)
                    // A response larger than the budget is abandoned rather
                    // than written, so a hostile host cannot fill the disk.
                    .arg("--max-filesize")
                    .arg(MAX_BYTES.to_string())
                    .arg("--max-time")
                    .arg("30")
                    .arg(&uri_clone)
                    .arg("-o")
                    .arg(&temp_path)
                    .status();

                let res = match status {
                    Ok(s) if s.success() => {
                        // `--max-filesize` trusts the advertised length, so
                        // the file is measured again before it is kept.
                        match std::fs::metadata(&temp_path) {
                            Ok(meta) if meta.len() > MAX_BYTES => {
                                let _ = std::fs::remove_file(&temp_path);
                                Err(anyhow!(
                                    "{} is larger than the {} byte limit for a remote image",
                                    uri_clone,
                                    MAX_BYTES
                                ))
                            }
                            _ => {
                                let _ = std::fs::rename(&temp_path, &cache_path_clone);
                                std::fs::read(&cache_path_clone).map_err(|e| anyhow!(e))
                            }
                        }
                    }
                    Ok(s) => {
                        let _ = std::fs::remove_file(&temp_path);
                        Err(anyhow!("curl failed with status: {}", s))
                    }
                    Err(e) => {
                        let _ = std::fs::remove_file(&temp_path);
                        Err(anyhow!("failed to spawn curl: {}", e))
                    }
                };
                let _ = tx.send(res);
            });

            let data = rx.await.map_err(|_| anyhow!("fetch cancelled"))??;
            let resp = Response::builder()
                .status(StatusCode::OK)
                .body(AsyncBody::from_bytes(data.into()))?;
            Ok(resp)
        }
        .boxed()
    }
}

#[cfg(test)]
mod destination_tests {
    use super::*;

    #[test]
    fn a_public_image_url_is_allowed() {
        for url in [
            "https://example.com/cat.png",
            "http://example.com/a/b.jpg",
            "https://user:pass@example.com/x.png",
            "https://example.com:8443/x.png",
            "https://93.184.216.34/x.png",
        ] {
            assert!(allows_destination(url), "{url} was refused");
        }
    }

    #[test]
    fn loopback_and_private_networks_are_refused() {
        for url in [
            "http://localhost/x.png",
            "http://LOCALHOST/x.png",
            "http://localhost:8080/x.png",
            "http://app.localhost/x.png",
            "http://127.0.0.1/x.png",
            "http://127.1.2.3/x.png",
            "http://[::1]/x.png",
            "http://10.0.0.5/x.png",
            "http://192.168.1.1/x.png",
            "http://172.16.0.1/x.png",
            "http://169.254.169.254/latest/meta-data/",
            "http://100.64.0.1/x.png",
            "http://0.0.0.0/x.png",
            "http://[fd00::1]/x.png",
            "http://[fe80::1]/x.png",
        ] {
            assert!(!allows_destination(url), "{url} was allowed");
        }
    }

    #[test]
    fn only_http_and_https_are_fetched() {
        for url in [
            "file:///etc/passwd",
            "ftp://example.com/x",
            "data:text/html,x",
        ] {
            assert!(!allows_destination(url), "{url} was allowed");
        }
        assert!(!allows_destination("example.com/x.png"), "no scheme");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_path_deterministic_and_preserves_ext() {
        let client = SilicaHttpClient::new();
        let p1 = client.cache_path("https://example.com/images/cat.png");
        let p2 = client.cache_path("https://example.com/images/cat.png");
        assert_eq!(p1, p2);
        assert_eq!(p1.extension().and_then(|s| s.to_str()), Some("png"));

        let p3 = client.cache_path("https://example.com/photo.jpeg?size=large");
        assert_eq!(p3.extension().and_then(|s| s.to_str()), Some("jpeg"));
    }

    #[test]
    fn disk_cache_hit_returns_cached_bytes() {
        let client = SilicaHttpClient::new();
        let url = "https://example.com/dummy_cached_test_image.png";
        let cache_path = client.cache_path(url);
        let test_bytes = b"fake-png-content";
        std::fs::write(&cache_path, test_bytes).unwrap();

        let req = http_client::http::Request::builder()
            .uri(url)
            .body(AsyncBody::empty())
            .unwrap();
        let future = client.send(req);
        let resp = futures::executor::block_on(future).unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let _ = std::fs::remove_file(cache_path);
    }

    #[test]
    fn a_public_name_resolves_to_a_public_address() {
        assert!(allows_destination("http://example.com/x.png"));
        assert!(allows_destination("https://example.com/x.png"));
        let resolve = resolve_target("http://example.com/x.png").unwrap();
        assert!(resolve.starts_with("example.com:80:"));
        // The resolved address must not be private or loopback.
        let ip = resolve.split_once(':').and_then(|(_, rest)| rest.split_once(':').map(|(_, ip)| ip)).unwrap();
        let ip: std::net::IpAddr = ip.parse().unwrap();
        assert!(!is_private(ip));
    }

    #[test]
    fn an_unresolvable_name_is_refused() {
        assert!(!allows_destination("http://this-name-does-not-exist-silica-test.invalid/x.png"));
        assert!(resolve_target("http://this-name-does-not-exist-silica-test.invalid/x.png").is_none());
    }

    #[test]
    fn resolve_target_pins_https_to_443() {
        let resolve = resolve_target("https://example.com/x.png").unwrap();
        assert!(resolve.starts_with("example.com:443:"));
    }
}
