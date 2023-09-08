use std::collections::HashMap;

use http::{request::Parts, Uri};
use hyper::{
    body::HttpBody, server::conn::Http, service::service_fn, Body, Method, Request, Response, Client, upgrade::Upgraded,
};

pub struct Hook<'a>{
    hook_target_host: [&'a str; 16],
    hook_target_path: [&'a str; 34],
    hook_domain_path: [&'a str; 5],
}

impl<'a>  Hook<'a> {
    pub fn new() ->Hook<'a>{
        // here is the code about init the url need to hook,maybe will alaways update
        let target_host = [
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
        let target_path = [
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
        
        let domain_list: [&str; 5] = [
            "music.163.com",
            "music.126.net",
            "iplay.163.com",
            "look.163.com",
            "y.163.com",
        ];
        Hook{
            hook_target_host: target_host,
            hook_target_path: target_path,
            hook_domain_path: domain_list,
        }
    }

    pub async fn check_host(&self,url:&str) -> bool {
        let check_host = self.hook_target_host;
        println!("检查----------------------------{:?}------", &url);
        if check_host.contains(&url){
            return true
        }else{
            return false;
        }
    }
    
    /**
     * @description: proxy request change the addr if request is not standard
     * @param {*} self
     * @param {Parts} parts
     * @return {*}
     */    
    pub async fn change_host_https(&self,parts:&Parts) -> Result<Uri, hyper::Error> {

        let send_host = "localhost";
        let send_port = 8082;
        let path_query = parts.uri.path_and_query()
            .map(|pq| pq.to_string().parse::<http::uri::PathAndQuery>().unwrap())
            .unwrap_or_else(|| http::uri::PathAndQuery::from_static("/"));                
        let new_uri = http::Uri::builder()
            .scheme("https")
            .authority(format!("{}:{}", send_host, send_port).as_str())
            .path_and_query(path_query)
            .build()
            .unwrap();
        Ok(new_uri)
        
    }

    pub async fn before_request(&self,mut req: Request<Body>){
        let request_method =  req.method().to_string();
        let request_path = req.uri().path().to_string();
        let check_host = self.hook_target_host;
        let check_request_url = req.uri().to_string();

        if (check_request_url.contains("music.163.com")){
            let mut proxy_map: HashMap<String, String> = HashMap::new();
            proxy_map.insert("decision".to_string(), "proxy".to_string());
            req.extensions_mut().insert(proxy_map);
        }

        let (mut parts, body) = req.into_parts();
        if let Some(Ok(host)) = parts.headers.get(http::header::HOST).map(|host| host.to_str())
        {
            let check_domain = self.hook_domain_path;
            if !check_domain.contains(&host){
                parts.uri = http::Uri::default();
            }
            
            if check_host.contains(&host) && request_method == "POST" && (request_path == "/api/linux/forward" || request_path.starts_with("/eapi/")){

            }
        }

    }
}

// const { req } = ctx;
//     console.log('---------------原始请求的url为',req.url)
// 	req.url =
// 		(req.url.startsWith('http://')
// 			? ''
// 			: (req.socket.encrypted ? 'https:' : 'http:') +
// 			  '//' +
// 			  (domainList.some((domain) =>
// 					(req.headers.host || '').endsWith(domain)
// 			  )
// 					? req.headers.host
// 					: null)) + req.url;
//     console.log('--------------------请求的url为',req.url)
// 	const url = parse(req.url);
// 	if (
// 		[url.hostname, req.headers.host].some((host) =>
// 			isHost(host, 'music.163.com')
// 		)
// 	)
// 		ctx.decision = 'proxy';
// 	if (
// 		[url.hostname, req.headers.host].some((host) =>
// 			hook.target.host.has(host)
// 		) &&
// 		req.method === 'POST' &&
// 		(url.path === '/api/linux/forward' || url.path.startsWith('/eapi/'))
// 	) {
// 		return request
// 			.read(req)
// 			.then((body) => (req.body = body))
// 			.then((body) => {
//                 // console.log('请求体-----------------',body)
// 				if ('x-napm-retry' in req.headers)
// 					delete req.headers['x-napm-retry'];
// 				req.headers['X-Real-IP'] = '118.88.88.88';
// 				if (
// 					req.url.includes('stream') ||
// 					req.url.includes('/eapi/cloud/upload/check')
// 				)
// 					return; // look living/cloudupload eapi can not be decrypted
// 				req.headers['Accept-Encoding'] = 'gzip, deflate'; // https://blog.csdn.net/u013022222/article/details/51707352
// 				if (body) {
// 					let data;
// 					const netease = {};
// 					netease.pad = (body.match(/%0+$/) || [''])[0];
// 					netease.forward = url.path === '/api/linux/forward';
                    
// 					if (netease.forward) {
// 						data = JSON.parse(
// 							crypto.linuxapi
// 								.decrypt(
// 									Buffer.from(
// 										body.slice(
// 											8,
// 											body.length - netease.pad.length
// 										),
// 										'hex'
// 									)
// 								)
// 								.toString()
// 						);
// 						netease.path = parse(data.url).path;
// 						netease.param = data.params;
// 					} else {
// 						data = crypto.eapi
// 							.decrypt(
// 								Buffer.from(
// 									body.slice(
// 										7,
// 										body.length - netease.pad.length
// 									),
// 									'hex'
// 								)
// 							)
// 							.toString()
// 							.split('-36cd479b6b5-');
// 						netease.path = data[0];
// 						netease.param = JSON.parse(data[1]);
//                         // console.log('netease------------------',netease)
// 					}
// 					netease.path = netease.path.replace(/\/\d*$/, '');
// 					ctx.netease = netease;
// 					// (netease.path, netease.param)

// 					if (netease.path === '/api/song/enhance/download/url')
// 						return pretendPlay(ctx);

// 					if (netease.path === '/api/song/enhance/download/url/v1')
// 						return pretendPlayV1(ctx);

// 					if (BLOCK_ADS) {
// 						if (netease.path.startsWith('/api/ad')) {
// 							ctx.error = new Error('ADs blocked.');
// 							ctx.decision = 'close';
// 						}
// 					}

// 					if (DISABLE_UPGRADE_CHECK) {
// 						if (
// 							netease.path.match(
// 								/^\/api(\/v1)?\/(android|ios|osx|pc)\/(upgrade|version)/
// 							)
// 						) {
// 							ctx.error = new Error('Upgrade check blocked.');
// 							ctx.decision = 'close';
// 						}
// 					}
// 				}
// 			})
// 			.catch(
// 				(error) =>
// 					error &&
// 					logger.error(
// 						error,
// 						`A error occurred in hook.request.before when hooking ${req.url}.`
// 					)
// 			);
// 	} else if (
// 		hook.target.host.has(url.hostname) &&
// 		(url.path.startsWith('/weapi/') || url.path.startsWith('/api/'))
// 	) {
// 		req.headers['X-Real-IP'] = '118.88.88.88';
// 		ctx.netease = {
// 			web: true,
// 			path: url.path
// 				.replace(/^\/weapi\//, '/api/')
// 				.split('?')
// 				.shift() // remove the query parameters
// 				.replace(/\/\d*$/, ''),
// 		};
// 	} else if (req.url.includes('package')) {
// 		try {
// 			const data = req.url.split('package/').pop().split('/');
// 			const url = parse(crypto.base64.decode(data[0]));
// 			const id = data[1].replace(/\.\w+/, '');
// 			req.url = url.href;
// 			req.headers['host'] = url.hostname;
// 			req.headers['cookie'] = null;
// 			ctx.package = { id };
// 			ctx.decision = 'proxy';
// 			// if (url.href.includes('google'))
// 			// 	return request('GET', req.url, req.headers, null, parse('http://127.0.0.1:1080'))
// 			// 	.then(response => (ctx.res.writeHead(response.statusCode, response.headers), response.pipe(ctx.res)))
// 		} catch (error) {
// 			ctx.error = error;
// 			ctx.decision = 'close';
// 		}
// 	}