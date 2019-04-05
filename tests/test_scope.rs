use actix_web_cute_codegen::{get, hook, scope};
use actix_http::HttpService;
use actix_http_test::TestServer;
use actix_web::{http, App, HttpResponse, Responder};
use futures::{Future, future};

use std::sync::atomic::{self, AtomicBool};

#[get("/outer_test")]
pub fn outer_test() -> impl Responder {
    HttpResponse::Ok()
}

static OUTER_INIT: AtomicBool = AtomicBool::new(false);
static INIT: AtomicBool = AtomicBool::new(false);
static HOOK_INIT: AtomicBool = AtomicBool::new(false);
static NOT_USED_HOOK_INIT: AtomicBool = AtomicBool::new(false);

pub fn outer_init_scope<P: 'static>(scope: actix_web::Scope<P>) -> actix_web::Scope<P> {
    assert!(!OUTER_INIT.swap(true, atomic::Ordering::Relaxed));
    scope
}

#[scope("/scope", hook="outer_init_scope", handler="outer_test")]
const mod_inner: () = {
    use super::*;

    #[get("/test")]
    pub fn test() -> impl Responder {
        HttpResponse::Ok()
    }

    #[get("/test_async")]
    pub fn auto_sync() -> impl Future<Item=HttpResponse, Error=actix_web::Error> {
        future::ok(HttpResponse::Ok().finish())
    }

    //Special member to act as hook
    pub fn init<P: 'static>(scope: actix_web::Scope<P>) -> actix_web::Scope<P> {
        INIT.store(true, atomic::Ordering::Relaxed);
        scope
    }

    #[hook]
    pub fn init_scope<P: 'static>(scope: actix_web::Scope<P>) -> actix_web::Scope<P> {
        HOOK_INIT.store(true, atomic::Ordering::Relaxed);
        scope
    }

    pub fn init_scope_unused<P: 'static>(scope: actix_web::Scope<P>) -> actix_web::Scope<P> {
        NOT_USED_HOOK_INIT.store(true, atomic::Ordering::Relaxed);
        scope
    }

    //TODO: once guard_fn will be public
    //#[actix_web_cute_codegen::guard]
    //fn guard_head(head: &actix_web::dev::RequestHead) -> bool {
    //    true
    //}

    pub fn default_resource<P: 'static>(res: actix_web::Resource<P>) -> actix_web::Resource<P> {
        res.to(|| HttpResponse::InternalServerError())
    }
};

#[test]
fn test_mod_inner() {
    let mut srv = TestServer::new(|| HttpService::new(App::new().service(mod_inner)));

    let request = srv.request(http::Method::GET, srv.url("/scope/test"));
    let response = srv.block_on(request.send()).unwrap();
    assert!(response.status().is_success());

    let request = srv.request(http::Method::GET, srv.url("/scope/outer_test"));
    let response = srv.block_on(request.send()).unwrap();
    assert!(response.status().is_success());

    let request = srv.request(http::Method::GET, srv.url("/scope/test_async"));
    let response = srv.block_on(request.send()).unwrap();
    assert!(response.status().is_success());

    let request = srv.request(http::Method::GET, srv.url("/unknown"));
    let response = srv.block_on(request.send()).unwrap();
    assert!(response.status().is_client_error());

    let request = srv.request(http::Method::GET, srv.url("/scope/unknown"));
    let response = srv.block_on(request.send()).unwrap();
    assert!(response.status().is_server_error());

    assert!(OUTER_INIT.load(atomic::Ordering::Relaxed));
    assert!(INIT.load(atomic::Ordering::Relaxed));
    assert!(HOOK_INIT.load(atomic::Ordering::Relaxed));
    assert!(!NOT_USED_HOOK_INIT.load(atomic::Ordering::Relaxed));
}
