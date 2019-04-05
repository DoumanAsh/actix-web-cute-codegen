use actix_web_cute_codegen::{get, scope, Scope};
use actix_http_test::TestServer;
use actix_http::HttpService;
use actix_web::{http, App, HttpResponse, Responder};
use futures::{Future, future};

use std::sync::atomic::{self, AtomicBool};

static INIT: AtomicBool = AtomicBool::new(false);
static HOOK_INIT: AtomicBool = AtomicBool::new(false);
static NOT_USED_HOOK_INIT: AtomicBool = AtomicBool::new(false);

#[get("/test")]
fn test() -> impl Responder {
    HttpResponse::Ok()
}

#[derive(Scope)]
#[path="/my_scope"]
pub struct MyScope {
    #[service]
    test: test,
    #[guard]
    guard: actix_web::guard::AnyGuard,
}

#[scope]
impl MyScope {
    fn new() -> Self {
        Self {
            test: test,
            guard: actix_web::guard::Any(actix_web::guard::Get()).or(actix_web::guard::Post()),
        }
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

    //TODO: once guard_fn will be public
    //#[actix_web_cute_codegen::guard]
    //fn guard_head(head: &actix_web::dev::RequestHead) -> bool {
    //    true
    //}

    pub fn init_scope_unused<P: 'static>(scope: actix_web::Scope<P>) -> actix_web::Scope<P> {
        NOT_USED_HOOK_INIT.store(true, atomic::Ordering::Relaxed);
        scope
    }

    #[get("/test_async", async)]
    pub fn test_async() -> impl Future<Item=HttpResponse, Error=actix_web::Error> {
        future::ok(HttpResponse::Ok().finish())
    }

    pub fn default_resource<P: 'static>(res: actix_web::Resource<P>) -> actix_web::Resource<P> {
        res.to(|| HttpResponse::InternalServerError())
    }
}

#[test]
fn test_my_scope() {
    let mut srv = TestServer::new(|| HttpService::new(App::new().service(MyScope::new())));

    let request = srv.request(http::Method::GET, srv.url("/my_scope/test"));
    let response = srv.block_on(request.send()).unwrap();
    assert!(response.status().is_success());

    let request = srv.request(http::Method::GET, srv.url("/my_scope/test"));
    let response = srv.block_on(request.send()).unwrap();
    assert!(response.status().is_success());

    let request = srv.request(http::Method::GET, srv.url("/unknown"));
    let response = srv.block_on(request.send()).unwrap();
    assert!(response.status().is_client_error());

    let request = srv.request(http::Method::GET, srv.url("/my_scope/unknown"));
    let response = srv.block_on(request.send()).unwrap();
    assert!(response.status().is_server_error());

    assert!(INIT.load(atomic::Ordering::Relaxed));
    assert!(HOOK_INIT.load(atomic::Ordering::Relaxed));
    assert!(!NOT_USED_HOOK_INIT.load(atomic::Ordering::Relaxed));

}
