use actix_http::HttpService;
use actix_http_test::TestServer;
use actix_web_cute_codegen::get;
use actix_web::{http, App, HttpResponse, Responder};
use futures::{Future, future};

//fn guard_head(head: &actix_web::dev::RequestHead) -> bool {
//    true
//}

//#[get("/test", guard="guard_head")]
#[get("/test")]
fn test() -> impl Responder {
    HttpResponse::Ok()
}

#[get("/test", async)]
fn async_test() -> impl Future<Item=HttpResponse, Error=actix_web::Error> {
    future::ok(HttpResponse::Ok().finish())
}

#[get("/test")]
fn auto_async() -> impl Future<Item=HttpResponse, Error=actix_web::Error> {
    future::ok(HttpResponse::Ok().finish())
}

#[get("/test")]
fn auto_sync() -> impl Future<Item=HttpResponse, Error=actix_web::Error> {
    future::ok(HttpResponse::Ok().finish())
}


#[test]
fn test_body() {
    let mut srv = TestServer::new(|| HttpService::new(App::new().service(test)));
    let request = srv.request(http::Method::GET, srv.url("/test"));
    let response = srv.block_on(request.send()).unwrap();
    assert!(response.status().is_success());

    let mut srv = TestServer::new(|| HttpService::new(App::new().service(auto_sync)));
    let request = srv.request(http::Method::GET, srv.url("/test"));
    let response = srv.block_on(request.send()).unwrap();
    assert!(response.status().is_success());
}

#[test]
fn test_async() {
    let mut srv = TestServer::new(|| HttpService::new(App::new().service(async_test)));

    let request = srv.request(http::Method::GET, srv.url("/test"));
    let response = srv.block_on(request.send()).unwrap();
    assert!(response.status().is_success());
}

#[test]
fn test_auto_async() {
    let mut srv = TestServer::new(|| HttpService::new(App::new().service(auto_async)));

    let request = srv.request(http::Method::GET, srv.url("/test"));
    let response = srv.block_on(request.send()).unwrap();
    assert!(response.status().is_success());
}
