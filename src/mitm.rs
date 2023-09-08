use crate::{
    ca::CertificateAuthority,
    handler::{CustomContextData, HttpHandler, MitmFilter},
    http_client::HttpClient,
    sni_reader::{
        read_sni_host_name_from_client_hello, HandshakeRecordReader, PrefixedReaderWriter,
        RecordingBufReader,
    },HOOK,
};
use http::{header, uri::Scheme, HeaderValue, Uri};
use hyper::{
    body::HttpBody, server::conn::Http, service::service_fn, Body, Method, Request, Response, Client, upgrade::Upgraded,
};
use url::Url;
use hyper_tls::{HttpsConnector, native_tls::{Certificate, TlsConnector}};
use log::*;
use std::{marker::PhantomData, sync::Arc, time::Duration, net::SocketAddr, io, str::FromStr};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    pin, time::timeout,
};
use tokio_rustls::TlsAcceptor;

/// Enum representing either an HTTP request or response.
#[derive(Debug)]
pub enum RequestOrResponse {
    Request(Request<Body>),
    Response(Response<Body>),
}

/// Context for HTTP requests and responses.
#[derive(Default, Debug)]
pub struct HttpContext<D: Default + Send + Sync> {
    pub uri: Option<Uri>,

    pub should_modify_response: bool,
    pub custom_data: D,
}

pub const HOOK_TARGET_HOST: [&str; 16] = [
    "music.163.com",
    "interface.music.163.com",
    "interface3.music.163.com",
    "apm.music.163.com",
    "apm3.music.163.com",
    "interface.music.163.com.163jiasu.com",
    "interface3.music.163.com.163jiasu.com",
    "112.13.119.18",
    "112.13.122.4",
    "117.147.199.59",
    "112.13.119.52",
    "112.13.119.51",
    "117.147.199.57",
    "117.161.69.87",
    "117.161.69.86",
    "59.111.19.33"

];

pub const HOOK_TARGET_PATH: [&str; 34] = [
    "/api/v3/playlist/detail",
    "/api/v3/song/detail",
    "/api/v6/playlist/detail",
    "/api/album/play",
    "/api/artist/privilege",
    "/api/album/privilege",
    "/api/v1/artist",
    "/api/v1/artist/songs",
    "/api/artist/top/song",
    "/api/v1/album",
    "/api/album/v3/detail",
    "/api/playlist/privilege",
    "/api/song/enhance/player/url",
    "/api/song/enhance/player/url/v1",
    "/api/song/enhance/download/url",
    "/api/song/enhance/privilege",
    "/batch",
    "/api/batch",
    "/api/v1/search/get",
    "/api/v1/search/song/get",
    "/api/search/complex/get",
    "/api/cloudsearch/pc",
    "/api/v1/playlist/manipulate/tracks",
    "/api/song/like",
    "/api/v1/play/record",
    "/api/playlist/v4/detail",
    "/api/v1/radio/get",
    "/api/v1/discovery/recommend/songs",
    "/api/v1/discovery/recommend/songs",
    "/api/usertool/sound/mobile/promote",
    "/api/usertool/sound/mobile/theme",
    "/api/usertool/sound/mobile/animationList",
    "/api/usertool/sound/mobile/all",
    "/api/usertool/sound/mobile/detail",
];

pub const HOOK_DOMAIN_LIST: [&str; 5] = [
    "music.163.com",
    "music.126.net",
    "iplay.163.com",
    "look.163.com",
    "y.163.com",
];

#[derive(Clone)]
pub(crate) struct MitmProxy<H, D>
where
    H: HttpHandler<D>,
    D: CustomContextData,
{
    pub ca: Arc<CertificateAuthority>,
    pub client: HttpClient,

    pub http_handler: Arc<H>,
    // pub mitm_filter: Arc<MitmFilter<D>>,

    pub custom_contex_data: PhantomData<D>,
}

impl<H, D> MitmProxy<H, D>
where
    H: HttpHandler<D>,
    D: CustomContextData,
{
    pub(crate) async fn proxy_req(
        self,
        req: Request<Body>,
    ) -> Result<Response<Body>, hyper::Error> {
        
        let res = if req.method() == Method::CONNECT {
                self.process_connect(req).await
        
        } else {
            self.process_request(req, Scheme::HTTP).await
        };

        match res {
            Ok(mut res) => {
                allow_all_cros(&mut res);
                Ok(res)
            }
            Err(err) => {
                error!("proxy request failed: {err:?}");
                Err(err)
            }
        }
    }

    async fn process_request(
        self,
        mut req: Request<Body>,
        scheme: Scheme,
    ) -> Result<Response<Body>, hyper::Error> {
        if req.uri().path().starts_with("/mitm/cert") {
            return Ok(self.get_cert_res());
        }

        let mut ctx = HttpContext {
            uri: None,
            should_modify_response: true,
            ..Default::default()
        };
        if req.version() == http::Version::HTTP_10 || req.version() == http::Version::HTTP_11 {
            let (mut parts, body) = req.into_parts();
            
            if let Some(Ok(authority)) = parts
                .headers
                .get(http::header::HOST)
                .map(|host| host.to_str())
            {
                let mut uri = parts.uri.into_parts();
                uri.scheme = Some(scheme.clone());
                uri.authority = authority.try_into().ok();
                parts.uri = Uri::from_parts(uri).expect("build uri");
            }

            req = Request::from_parts(parts, body);
        };
        // }

        let mut req = match self.http_handler.handle_request(&mut ctx, req).await {
            RequestOrResponse::Request(req) => req,
            RequestOrResponse::Response(res) => return Ok(res),
        };

        {
            let header_mut = req.headers_mut();
            header_mut.remove(http::header::HOST);
            header_mut.remove(http::header::ACCEPT_ENCODING);
            header_mut.remove(http::header::CONTENT_LENGTH);
        }

        let res = match self.client {
            HttpClient::Proxy(client) => client.request(req).await?,
            HttpClient::Https(client) => client.request(req).await?,
        };

        let mut res = self.http_handler.handle_response(&mut ctx, res).await;
        let length = res.size_hint().lower();

        {
            let header_mut = res.headers_mut();

            if let Some(content_length) = header_mut.get_mut(http::header::CONTENT_LENGTH) {
                *content_length = HeaderValue::from_str(&length.to_string()).unwrap();
            }

            // Remove `Strict-Transport-Security` to avoid HSTS
            // See: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Strict-Transport-Security
            header_mut.remove(header::STRICT_TRANSPORT_SECURITY);
        }

        Ok(res)
    }

    /**
     * @description: proxy connect request
     * @param {*} self
     * @param {*} mut
     * @return {*}
     */    
    async fn process_connect(self, mut req: Request<Body>) -> Result<Response<Body>, hyper::Error> {   
        let (mut parts, body) = req.into_parts();
        if let Some(Ok(host)) = parts.headers.get(http::header::HOST).map(|host| host.to_str())
            {
                let check_host = format!("https://{}" ,host);
                if let Ok(url) = Url::parse(&check_host) {
                    let hostname = url.host_str().unwrap_or("N/A");
                    let check_result = HOOK.check_host(hostname).await;
                    // println!("请求----------------------------{:?}-------{}", &hostname,check_result);
                    // if match need to modify
                    if check_result {
                        parts.uri = HOOK.change_host_https(&parts).await.unwrap();
                    }
                }
                
                
            }
            req = Request::from_parts(parts, body);
            tokio::task::spawn(async move {
                let remote_addr = host_addr(req.uri()).unwrap();
                let upgraded = hyper::upgrade::on(req).await.unwrap();
                tunnel(upgraded, remote_addr).await
            });
        Ok(Response::new(Body::empty()))
    }


    pub async fn serve_tls<IO: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        self,
        mut stream: IO,
    ) {
        // Read SNI hostname.
        let recording_reader = RecordingBufReader::new(&mut stream);
        let read_buf = recording_reader.buf();
        let client_stream = PrefixedReaderWriter::new(stream, read_buf);
        
        let server_config = self.ca.clone().gen_server_config();
        match TlsAcceptor::from(server_config).accept(client_stream).await {
            Ok(stream) => {
                if let Err(e) = Http::new()
                    .http1_preserve_header_case(true)
                    .http1_title_case_headers(true)
                    .serve_connection(
                        stream,
                        service_fn(|req| {
                            println!("Received https request for URL: {}", req.uri()); // 打印URL
                            self.clone().process_request(req, Scheme::HTTPS)
                        }),
                    )
                    .with_upgrades()
                    .await
                {
                    let e_string = e.to_string();
                    if !e_string.starts_with("error shutting down connection") {
                        debug!("res:: {}", e);
                    }
                }
            }
            Err(err) => {
                error!("Tls accept failed: {err}")
            }
        }
    }

    pub async fn serve_stream<S>(self, stream: S) -> Result<(), hyper::Error>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        Http::new()
            .http1_preserve_header_case(true)
            .http1_title_case_headers(true)
            .serve_connection(stream, service_fn(|req| 
                {   
                    self.clone().proxy_req(req)
                }
                
            ))
            .with_upgrades()
            .await
    }

    fn get_cert_res(&self) -> hyper::Response<Body> {
        Response::builder()
            .header(
                http::header::CONTENT_DISPOSITION,
                "attachment; filename=good-mitm.crt",
            )
            .header(http::header::CONTENT_TYPE, "application/octet-stream")
            .status(http::StatusCode::OK)
            .body(Body::from(self.ca.clone().get_cert()))
            .unwrap()
    }
}

fn allow_all_cros(res: &mut Response<Body>) {
    let header_mut = res.headers_mut();
    let all = HeaderValue::from_str("*").unwrap();
    header_mut.insert(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, all.clone());
    header_mut.insert(http::header::ACCESS_CONTROL_ALLOW_METHODS, all.clone());
    header_mut.insert(http::header::ACCESS_CONTROL_ALLOW_METHODS, all);
}

fn host_addr(uri: &http::Uri) -> Option<String> {
    uri.authority().map(|auth| auth.to_string())
}

async fn tunnel<A>(mut client_stream: A, addr: String) -> std::io::Result<()>
where
    A: AsyncRead + AsyncWrite + Unpin,
{
//     println!("2231231---地址-----{}--",addr);
    let mut server = match TcpStream::connect(addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error connecting: {}", e);
            return Err(e);
        }
    };

    match tokio::io::copy_bidirectional(&mut client_stream, &mut server).await {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Error in bidirectional copy: {}", e);
            return Err(e);
        }
    }

    Ok(())
}


