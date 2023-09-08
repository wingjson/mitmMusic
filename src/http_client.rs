use crate::error::Error;
use hyper::{client::HttpConnector, Client};
use hyper_proxy::{Proxy as UpstreamProxy, ProxyConnector};
use rustls::client::{ServerCertVerified, ServerCertVerifier};
use std::time::SystemTime;

cfg_if::cfg_if! {
    if #[cfg(feature = "request-native-tls")] {
        use hyper_tls::{HttpsConnector, native_tls::TlsConnector};
    } else {
        use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
        use rustls::ClientConfig;
        use std::sync::Arc;
    }
}

#[derive(Clone)]
pub enum HttpClient {
    Proxy(Client<ProxyConnector<HttpsConnector<HttpConnector>>>),
    Https(Client<HttpsConnector<HttpConnector>>),
}

pub fn gen_client(upstream_proxy: Option<UpstreamProxy>) -> Result<HttpClient, Error> {
    cfg_if::cfg_if! {
        if #[cfg(feature = "request-native-tls")] {
            let https = {
                let tls = TlsConnector::builder()
                    .danger_accept_invalid_certs(true)
                    .danger_accept_invalid_hostnames(true)
                    .disable_built_in_roots(true)
                    .build()?;
                let mut http = HttpConnector::new();
                http.enforce_http(false);
                HttpsConnector::from((http, tls.into()))
            };
        } else {
            let https = {
                let https_builder = HttpsConnectorBuilder::new()
                    .with_tls_config({
                        let cert_resolver = Arc::new(TrustAllCertVerifier::default());
                        ClientConfig::builder()
                            .with_safe_defaults()
                            .with_custom_certificate_verifier(cert_resolver)
                            .with_no_client_auth()
                    })
                    .https_or_http()
                    .enable_http1();
                #[cfg(feature = "h2")]
                let https_builder = https_builder.enable_http2();

                https_builder.build()
            };
        }
    }

    if let Some(proxy) = upstream_proxy {
        let connector = ProxyConnector::from_proxy(https, proxy)?;
        return Ok(HttpClient::Proxy(
            Client::builder()
                .http1_title_case_headers(true)
                .http1_preserve_header_case(true)
                .build(connector),
        ));
    } else {
        Ok(HttpClient::Https(
            Client::builder()
                .http1_title_case_headers(true)
                .http1_preserve_header_case(true)
                .build(https),
        ))
    }
}

#[derive(Default)]
struct TrustAllCertVerifier;

impl ServerCertVerifier for TrustAllCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _n_ow: SystemTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
}
