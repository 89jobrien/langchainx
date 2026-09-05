use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use async_trait::async_trait;
use url::{Host, Url};

use super::OpenApiToolkitError;

const MAX_AUTHORIZED_ADDRESSES: usize = 16;

/// Exact HTTP origin used to bind authorization and credentials.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HttpOrigin {
    scheme: String,
    host: String,
    port: u16,
}

impl HttpOrigin {
    /// Extracts and canonicalizes the scheme, host, and effective port.
    pub fn try_from_url(url: &Url) -> Result<Self, OpenApiToolkitError> {
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(OpenApiToolkitError::EgressDenied(
                "scheme is not HTTP(S)".into(),
            ));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(OpenApiToolkitError::EgressDenied(
                "URL userinfo is forbidden".into(),
            ));
        }
        let host = match url
            .host()
            .ok_or_else(|| OpenApiToolkitError::EgressDenied("URL has no host".into()))?
        {
            Host::Domain(host) => host.to_ascii_lowercase(),
            Host::Ipv4(host) => host.to_string(),
            Host::Ipv6(host) => host.to_string(),
        };
        let port = url
            .port_or_known_default()
            .ok_or_else(|| OpenApiToolkitError::EgressDenied("URL has no effective port".into()))?;
        Ok(Self {
            scheme: scheme.to_owned(),
            host,
            port,
        })
    }

    /// Returns the canonical scheme.
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// Returns the canonical host.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the effective port.
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// An origin authorized together with its pinned resolved addresses.
#[derive(Clone, Debug)]
pub struct AuthorizedOrigin {
    origin: HttpOrigin,
    addresses: Vec<IpAddr>,
}

impl AuthorizedOrigin {
    /// Creates an authorization result with at least one pinned address.
    pub fn new(origin: HttpOrigin, addresses: Vec<IpAddr>) -> Result<Self, OpenApiToolkitError> {
        let mut seen = HashSet::with_capacity(addresses.len().min(MAX_AUTHORIZED_ADDRESSES));
        let mut deduplicated = Vec::with_capacity(MAX_AUTHORIZED_ADDRESSES);
        for address in addresses {
            if seen.insert(address) {
                if deduplicated.len() == MAX_AUTHORIZED_ADDRESSES {
                    return Err(OpenApiToolkitError::EgressDenied(
                        "origin resolved to too many addresses".into(),
                    ));
                }
                deduplicated.push(address);
            }
        }
        if deduplicated.is_empty() {
            return Err(OpenApiToolkitError::EgressDenied(
                "origin resolved to no addresses".into(),
            ));
        }
        deduplicated.shrink_to_fit();
        Ok(Self {
            origin,
            addresses: deduplicated,
        })
    }

    /// Returns the exact authorized origin.
    pub fn origin(&self) -> &HttpOrigin {
        &self.origin
    }

    /// Returns the pinned addresses.
    pub fn addresses(&self) -> &[IpAddr] {
        &self.addresses
    }
}

/// Safe operation metadata supplied to execution policies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationContext {
    method: String,
    path: String,
    operation_id: String,
}

impl OperationContext {
    pub(crate) fn new(method: &str, path: &str, operation_id: &str) -> Self {
        Self {
            method: method.to_owned(),
            path: path.to_owned(),
            operation_id: operation_id.to_owned(),
        }
    }

    /// Returns the uppercase HTTP method.
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Returns the path template without its server origin.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the specification operation identifier.
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
}

/// Authorizes and pins outbound HTTP origins.
#[async_trait]
pub trait HttpEgressPolicy: Send + Sync {
    /// Authorizes a URL and returns the addresses the transport must use.
    async fn authorize(&self, url: &Url) -> Result<AuthorizedOrigin, OpenApiToolkitError>;
}

/// Authorizes execution of a generated OpenAPI operation.
#[async_trait]
pub trait OperationPolicy: Send + Sync {
    /// Returns success only when the operation may execute.
    async fn authorize(&self, operation: &OperationContext) -> Result<(), OpenApiToolkitError>;
}

/// Public-network policy that rejects local and special-purpose destinations.
#[derive(Clone, Copy, Debug, Default)]
pub struct PublicHttpEgressPolicy;

#[async_trait]
impl HttpEgressPolicy for PublicHttpEgressPolicy {
    async fn authorize(&self, url: &Url) -> Result<AuthorizedOrigin, OpenApiToolkitError> {
        let origin = HttpOrigin::try_from_url(url)?;
        if !matches!(origin.port(), 80 | 443) {
            return Err(OpenApiToolkitError::EgressDenied(
                "unsafe destination port".into(),
            ));
        }
        let resolved = tokio::net::lookup_host((origin.host(), origin.port()))
            .await
            .map_err(|_| OpenApiToolkitError::EgressDenied("host resolution failed".into()))?;
        let mut addresses = Vec::with_capacity(MAX_AUTHORIZED_ADDRESSES);
        let mut seen = HashSet::with_capacity(MAX_AUTHORIZED_ADDRESSES);
        for address in resolved.map(|address| address.ip()) {
            if seen.insert(address) {
                if addresses.len() == MAX_AUTHORIZED_ADDRESSES {
                    return Err(OpenApiToolkitError::EgressDenied(
                        "host resolves to too many addresses".into(),
                    ));
                }
                addresses.push(address);
            }
        }
        if addresses.iter().any(|address| !is_global_address(*address)) {
            return Err(OpenApiToolkitError::EgressDenied(
                "host resolves to a non-global address".into(),
            ));
        }
        AuthorizedOrigin::new(origin, addresses)
    }
}

/// Execution policy that allows every generated method.
#[derive(Clone, Copy, Debug, Default)]
pub struct AllowAllOperations;

#[async_trait]
impl OperationPolicy for AllowAllOperations {
    async fn authorize(&self, _operation: &OperationContext) -> Result<(), OpenApiToolkitError> {
        Ok(())
    }
}

/// Execution policy that allows only GET, HEAD, and OPTIONS.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReadOnlyOperations;

#[async_trait]
impl OperationPolicy for ReadOnlyOperations {
    async fn authorize(&self, operation: &OperationContext) -> Result<(), OpenApiToolkitError> {
        if matches!(operation.method(), "GET" | "HEAD" | "OPTIONS") {
            return Ok(());
        }
        Err(OpenApiToolkitError::MutatingOperationDenied(
            operation.operation_id().to_owned(),
        ))
    }
}

fn is_global_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_global_ipv4(address),
        IpAddr::V6(address) => {
            if let Some(mapped) = address.to_ipv4_mapped() {
                return is_global_ipv4(mapped);
            }
            is_global_ipv6(address)
        }
    }
}

fn is_global_ipv4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    !(address.is_unspecified()
        || address.is_loopback()
        || address.is_private()
        || address.is_link_local()
        || address.is_multicast()
        || address.is_broadcast()
        || octets[0] == 0
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
        || (octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
        || (octets[0] == 192 && octets[1] == 88 && octets[2] == 99)
        || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
        || (octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
        || (octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
        || octets[0] >= 240)
}

fn is_global_ipv6(address: Ipv6Addr) -> bool {
    let segments = address.segments();
    let globally_routed = (segments[0] & 0xe000) == 0x2000;
    globally_routed
        && !address.is_unspecified()
        && !address.is_loopback()
        && !address.is_multicast()
        && (segments[0] & 0xfe00) != 0xfc00
        && (segments[0] & 0xffc0) != 0xfe80
        && !(segments[0] == 0x0064 && matches!(segments[1], 0xff9b))
        && !(segments[0] == 0x2001 && segments[1] <= 0x01ff)
        && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
        && !(segments[0] == 0x2001 && segments[1] == 0x0002)
        && segments[0] != 0x2002
        && !(segments[0] >= 0x3ff0 && segments[0] <= 0x3fff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_equality_uses_effective_port_and_canonical_host() {
        let first =
            HttpOrigin::try_from_url(&Url::parse("https://EXAMPLE.com/a").unwrap()).unwrap();
        let second =
            HttpOrigin::try_from_url(&Url::parse("https://example.com:443/b").unwrap()).unwrap();
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn public_policy_rejects_non_global_mapped_and_unsafe_urls() {
        let policy = PublicHttpEgressPolicy;
        for url in [
            "http://127.0.0.1/",
            "http://10.0.0.1/",
            "http://[::ffff:127.0.0.1]/",
            "ftp://8.8.8.8/",
            "http://user@8.8.8.8/",
            "http://8.8.8.8:22/",
        ] {
            assert!(
                policy.authorize(&Url::parse(url).unwrap()).await.is_err(),
                "{url}"
            );
        }
    }

    #[tokio::test]
    async fn public_policy_allows_a_global_literal_address() {
        assert!(
            PublicHttpEgressPolicy
                .authorize(&Url::parse("https://8.8.8.8/").unwrap())
                .await
                .is_ok()
        );
    }

    #[test]
    fn authorized_origin_deduplicates_and_caps_addresses() {
        let origin = HttpOrigin::try_from_url(&Url::parse("https://example.com").unwrap()).unwrap();
        let duplicate = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        let authorized = AuthorizedOrigin::new(origin.clone(), vec![duplicate; 32]).unwrap();
        assert_eq!(authorized.addresses(), &[duplicate]);

        let addresses = (1..=17)
            .map(|last| IpAddr::V4(Ipv4Addr::new(8, 8, 8, last)))
            .collect();
        assert!(AuthorizedOrigin::new(origin, addresses).is_err());
    }
}
