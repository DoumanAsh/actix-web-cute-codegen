#![recursion_limit = "512"]
//! Actix-web codegen module
//!
//! Generators for routes and scopes
//!
//! ## Route
//!
//! Macros:
//!
//! - [handler](attr.handler.html)
//! - [get](attr.get.html)
//! - [post](attr.post.html)
//! - [put](attr.put.html)
//! - [delete](attr.delete.html)
//!
//! ### Attributes:
//!
//! - `"path"` - Raw literal string with path for which to register handle. Mandatory.
//! - `async` - Attribute to indicate that registered function is asynchronous.
//! - `guard="function_name"` - Registers function as guard using `actix_web::guard::fn_guard`
//!
//! ## Scope
//!
//! Macros:
//!
//! - [scope](attr.scope.html)
//!
//! ### Attributes:
//!
//! - `"path"` - Raw literal string with path for which to register handle. Mandatory.
//! - `guard="function_name"` - Registers function as guard using `actix_web::guard::fn_guard`
//!
//! ## Notes
//!
//! Function name can be specified as any expression that is going to be accessible to the generate
//! code (e.g `my_guard` or `my_module::my_guard`)
//!
//! ## Example:
//!
//! ```rust
//! use actix_web::HttpResponse;
//! use actix_web_cute_codegen::get;
//! use futures::{future, Future};
//!
//! #[get("/test", async)]
//! fn async_test() -> impl Future<Item=HttpResponse, Error=actix_web::Error> {
//!     future::ok(HttpResponse::Ok().finish())
//! }
//! ```

extern crate proc_macro;

mod route;
mod scope;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Creates route handler without attached method guard.
///
/// Syntax: `#[handler("path"[, attributes])]`
///
/// ## Attributes:
///
/// - `"path"` - Raw literal string with path for which to register handler. Mandatory.
/// - `async` - Attribute to indicate that registered function is asynchronous.
/// - `guard="function_name"` - Registers function as guard using `actix_web::guard::fn_guard`
#[proc_macro_attribute]
pub fn handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as syn::AttributeArgs);
    let gen = route::Args::new(&args, input, route::GuardType::Get);
    gen.generate()
}

/// Creates route handler with `GET` method guard.
///
/// Syntax: `#[get("path"[, attributes])]`
///
/// Attributes are the same as in [handler](attr.handler.html)
#[proc_macro_attribute]
pub fn get(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as syn::AttributeArgs);
    let gen = route::Args::new(&args, input, route::GuardType::Get);
    gen.generate()
}

/// Creates route handler with `POST` method guard.
///
/// Syntax: `#[post("path"[, attributes])]`
///
/// Attributes are the same as in [handler](attr.handler.html)
#[proc_macro_attribute]
pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as syn::AttributeArgs);
    let gen = route::Args::new(&args, input, route::GuardType::Post);
    gen.generate()
}

/// Creates route handler with `PUT` method guard.
///
/// Syntax: `#[put("path"[, attributes])]`
///
/// Attributes are the same as in [handler](attr.handler.html)
#[proc_macro_attribute]
pub fn put(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as syn::AttributeArgs);
    let gen = route::Args::new(&args, input, route::GuardType::Put);
    gen.generate()
}

/// Creates route handler with `DELETE` method guard.
///
/// Syntax: `#[delete("path"[, attributes])]`
///
/// Attributes are the same as in [handler](attr.handler.html)
#[proc_macro_attribute]
pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as syn::AttributeArgs);
    let gen = route::Args::new(&args, input, route::GuardType::Delete);
    gen.generate()
}

/// Generates scope
///
/// Syntax: `#[scope("path"[, attributes])]`
///
/// Due to current limitation it cannot be applied to modules themself.
/// Instead one should create const variable that contains module code.
///
/// ## Attributes:
///
/// - `"path"` - Raw literal string with path for which to register handler. Mandatory.
/// - `hook="function_name"` - Registers function to be run on scope before registering everything else.
/// - `guard="function_name"` - Registers function as guard using `actix_web::guard::fn_guard`
/// - `hanlder="function_name"` - Registers route hanlder as part of scope.
///
/// ## Special members:
///
/// - `#[hook]` functions - are going to be run on scope before registering everything else
/// - `#[guard]` functions - specifies function to be passed to `guard_fn`.
/// - `init` - Scope initialization function. Used as `hook`
/// - `default_resource` - function will be used as default method to the scope.
///
/// # Example
///
/// ```rust
/// use actix_web_cute_codegen::{scope};
///
/// #[scope("/scope")]
/// const mod_inner: () = {
///     use actix_web_cute_codegen::{get, hook};
///     use actix_web::{HttpResponse, Responder};
///     use futures::{Future, future};
///
///     #[get("/test")]
///     pub fn test() -> impl Responder {
///         HttpResponse::Ok()
///     }
///
///     #[get("/test_async")]
///     pub fn auto_sync() -> impl Future<Item=HttpResponse, Error=actix_web::Error> {
///         future::ok(HttpResponse::Ok().finish())
///     }
///
///     ///Special function to initialize scope. Called as hook before routes registration.
///     pub fn init<P: 'static>(scope: actix_web::Scope<P>) -> actix_web::Scope<P> {
///         scope
///     }
///
///     pub fn default_resource<P: 'static>(res: actix_web::Resource<P>) -> actix_web::Resource<P> {
///         res.to(|| HttpResponse::InternalServerError())
///     }
///
/// };
/// ```
///
/// # Note
///
/// Internally the macro generate struct with name of scope (e.g. `mod_inner`)
/// And create public module as `<name>_scope`
#[proc_macro_attribute]
pub fn scope(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as syn::AttributeArgs);
    match args.len() {
        0 => scope::attr::ImplScope::new(input).generate(),
        _ => scope::attr::Args::new(&args, input).generate(),
    }
}

///Marks function as guard
#[proc_macro_attribute]
pub fn guard(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

///Marks function as hook.
///
///Depending on context it allows to run custom code on item
///before it registers stuff
#[proc_macro_attribute]
pub fn hook(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

///Generates Scope
///
///Similar to [scope](attr.scope.html) macro
///
///## Example
///
///```rust
///use futures::{Future, future};
///use actix_web_cute_codegen::{get, scope, Scope};
///
///#[get("/test")]
///fn test() -> impl actix_web::Responder {
///    actix_web::HttpResponse::Ok()
///}
///
///#[derive(Scope)]
///#[path="/my_scope"]
///pub struct MyScope {
///    #[service]
///    test: test,
///    #[guard]
///    guard: actix_web::guard::AnyGuard,
///}
///
///#[scope]
///impl MyScope {
///    fn new() -> Self {
///        Self {
///            test: test,
///            guard: actix_web::guard::Any(actix_web::guard::Get()).or(actix_web::guard::Post()),
///        }
///    }
///
///    //Special member to act as hook
///    pub fn init<P: 'static>(scope: actix_web::Scope<P>) -> actix_web::Scope<P> {
///        scope
///    }
///
///    //Mark any function as initialization of scope
///    #[hook]
///    pub fn init_scope<P: 'static>(scope: actix_web::Scope<P>) -> actix_web::Scope<P> {
///        scope
///    }
///
///
///    pub fn init_scope_unused<P: 'static>(scope: actix_web::Scope<P>) -> actix_web::Scope<P> {
///        scope
///    }
///
///    #[get("/test_async")]
///    pub fn test_async() -> impl Future<Item=actix_web::HttpResponse, Error=actix_web::Error> {
///        future::ok(actix_web::HttpResponse::Ok().finish())
///    }
///
///    pub fn default_resource<P: 'static>(res: actix_web::Resource<P>) -> actix_web::Resource<P> {
///        res.to(|| actix_web::HttpResponse::InternalServerError())
///    }
///}
///```
#[proc_macro_derive(Scope, attributes(path, service, guard))]
pub fn parser_derive(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let gen = scope::derive::Args::new(ast);
    gen.generate()
}
