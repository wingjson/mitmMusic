use async_trait::async_trait;
use hyper::{header, Body, Request, Response};
use log::info;
use crate::handler::{CustomContextData, HttpHandler};
use crate::mitm::{HttpContext, RequestOrResponse};

#[derive(Clone)]
pub struct UmuHttpHandler;

impl UmuHttpHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Default, Clone)]
pub struct UmuHandlerCtx;

impl CustomContextData for UmuHandlerCtx {}

#[async_trait]
impl HttpHandler<UmuHandlerCtx> for UmuHttpHandler {
    async fn handle_request(
        &self,
        ctx: &mut HttpContext<UmuHandlerCtx>,
        req: Request<Body>,
    ) -> RequestOrResponse {
        ctx.uri = Some(req.uri().clone());
        // println!("\n请求连接是{}\n", req.uri());
        println!("\n request-----------------{:?}-------------------\n", req.method());

        // remove accept-encoding to avoid encoded body
        let mut req = req;
        req.headers_mut().remove(header::ACCEPT_ENCODING);

        RequestOrResponse::Request(req)
    }

    async fn handle_response(
        &self,
        ctx: &mut HttpContext<UmuHandlerCtx>,
        res: Response<Body>,
    ) -> Response<Body> {
        println!("\n 返回的是{:?}-------------------\n", res);
        let uri = ctx.uri.as_ref().unwrap();
        let content_type = match res.headers().get(header::CONTENT_TYPE) {
            Some(content_type) => content_type.to_str().unwrap_or_default(),
            None => "unknown",
        };
        info!(
            "[Response] {} {} {}",
            res.status(),
            uri.host().unwrap_or_default(),
            content_type
        );

        res
    }
}
