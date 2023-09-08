/*
 * @Date: 2023-09-06 16:10:55
 * @LastEditors: WWW
 * @LastEditTime: 2023-09-08 14:45:20
 * @FilePath: \mitmMusic\src\main.rs
 */
/*
 * @Date: 2023-09-06 16:10:55
 * @LastEditors: WWW
 * @LastEditTime: 2023-09-07 14:12:04
 * @FilePath: \mitmMusic\src\main.rs
 */
/*
 * @Date: 2023-09-06 16:10:55
 * @LastEditors: WWW
 * @LastEditTime: 2023-09-07 14:00:40
 * @FilePath: \mitmMusic\src\main.rs
 */
/*
 * @Date: 2023-08-28 14:38:37
 * @LastEditors: WWW
 * @LastEditTime: 2023-09-06 16:59:34
 * @FilePath: \mitmMusic\src\main.rs
 */
mod umuhandler;
mod mitm;
mod ca;
mod proxy;
mod error;
mod hook;
mod handler;
mod http_client;
mod sni_reader;
mod utilts;
mod config;

use anyhow::Ok;
use ca::CertificateAuthority;
use config::{read_config,Config};
use env_logger::Env;
use hyper_proxy::Intercept;
use hook::Hook;
use log::*;
use lazy_static::lazy_static;
use std::net::SocketAddr;
use proxy::Proxy;
use umuhandler::UmuHttpHandler;
use utilts::shutdown_signal;

pub struct Opts {
    proxy: Option<String>,
}
lazy_static! {
    pub static ref HOOK: Hook<'static> = Hook::new();
    pub static ref CONFIG: Config = read_config().unwrap();
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(Env::default().default_filter_or("debug"));
        let config = &CONFIG;
        let ca = CertificateAuthority::load_ca(&config.private, &config.cert).unwrap();
        info!("Https Proxy listen on: http://{}", config.https);
        info!("Http Proxy listen on: https://{}", config.http);
        
        let opts = Opts {
            proxy: None,
        };
        // http port
        let http_address: SocketAddr = config.http.parse().unwrap();
        let http_handler = UmuHttpHandler::new();
        let http_proxy = Proxy::builder()
            .ca(ca.clone())
            .listen_addr(http_address)
            .upstream_proxy(
                opts.proxy
                    .clone()
                    .map(|proxy| hyper_proxy::Proxy::new(Intercept::All, proxy.parse().unwrap())),
            )
            .shutdown_signal(shutdown_signal())
            .handler(http_handler.clone())
            .build();

        tokio::spawn(http_proxy.start_http_proxy());
        
        // https port
        let https_address: SocketAddr =config.https.parse().unwrap();
        let https_handler = http_handler.clone();
        let https_proxy = Proxy::builder()
            .ca(ca.clone())
            .listen_addr(https_address)
            .upstream_proxy(
                opts.proxy
                    .clone()
                    .map(|proxy| hyper_proxy::Proxy::new(Intercept::All, proxy.parse().unwrap())),
            )
            .shutdown_signal(shutdown_signal())
            .handler(https_handler.clone())
            .build();

        tokio::spawn(https_proxy.start_https_proxy());

        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for event");
        let _ = Ok(());
    
}


