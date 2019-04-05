use quote::{quote};
use proc_macro::TokenStream;

use crate::route::{GuardType};
use std::{mem, fmt};
use std::collections::HashSet;

#[derive(Default)]
struct Items {
    guards: Vec<String>,
    hooks: Vec<String>,
    handlers: Vec<String>,
}

struct ScopeItems {
    handlers: Vec<String>,
    guards: Vec<String>,
    hooks: Vec<String>,
    default: Option<String>,
    raw_routes: Vec<syn::ItemFn>
}

impl ScopeItems {
    pub fn from_items(items: &[syn::Item]) -> Self {
        let mut handlers = Vec::new();
        let mut guards = Vec::new();
        let mut hooks = Vec::new();
        let mut default = None;
        let mut raw_routes = Vec::new();

        for item in items {
            match item {
                syn::Item::Fn(ref fun) => {
                    if fun.ident == "default_resource" {
                        if default.is_some() {
                            panic!("Second 'default_resource' in scope! You cannot have more than one default resource");
                        }

                        default = Some(format!("{}", fun.ident));
                        continue;
                    } else if fun.ident == "init" {
                        hooks.push(fun.ident.to_string());
                        continue;
                    }

                    for attr in fun.attrs.iter() {
                        for bound in attr.path.segments.iter() {
                            if bound.ident == "get" || bound.ident == "put" || bound.ident == "handler" || bound.ident == "delete" || bound.ident == "post" {
                                handlers.push(format!("{}", fun.ident));
                                raw_routes.push(fun.clone());
                                break;
                            } else if bound.ident == "guard" {
                                guards.push(format!("{}", fun.ident));
                                break;
                            } else if bound.ident == "hook" {
                                hooks.push(format!("{}", fun.ident));
                                break;
                            }
                        }
                    }
                },
                _ => continue,
            }
        }

        Self {
            handlers,
            guards,
            hooks,
            default,
            raw_routes,
        }
    }
}

pub struct Args {
    ast: syn::ItemConst,
    name: syn::Ident,
    path: String,
    items: Items,
    scope_items: ScopeItems,
}

impl Args {
    pub fn new(args: &Vec<syn::NestedMeta>, input: TokenStream) -> Self {
        if args.is_empty() {
            panic!("invalid server definition, expected: #[scope(\"some path\")]");
        }

        let ast: syn::ItemConst = syn::parse(input).expect("Parse input as module");
        //TODO: we should change it to mod once supported on stable
        //let ast: syn::ItemMod = syn::parse(input).expect("Parse input as module");
        let name = ast.ident.clone();

        let mut items = Vec::new();
        match ast.expr.as_ref() {
            syn::Expr::Block(expr) => for item in expr.block.stmts.iter() {
                match item {
                    syn::Stmt::Item(ref item) => items.push(item.clone()),
                    _ => continue,
                }
            },
            _ => panic!("Scope should containt only code block { }"),
        }

        let scope_items = ScopeItems::from_items(&items);
        let mut items = Items::default();

        let mut path = None;
        for arg in args {
            match arg {
                syn::NestedMeta::Literal(syn::Lit::Str(ref fname)) => {
                    if path.is_some() {
                        panic!("Multiple paths specified! Should be only one!")
                    }
                    let fname = quote!(#fname).to_string();
                    path = Some(fname.as_str()[1..fname.len() - 1].to_owned())
                },
                syn::NestedMeta::Meta(syn::Meta::NameValue(ident)) => match ident.ident.to_string().to_lowercase().as_str() {
                    "guard" => match ident.lit {
                        syn::Lit::Str(ref text) => items.guards.push(text.value()),
                        _ => panic!("Attribute guard expects literal string!"),
                    },
                    "hook" => match ident.lit {
                        syn::Lit::Str(ref text) => items.hooks.push(text.value()),
                        _ => panic!("Attribute hook expects literal string!"),
                    },
                    "handler" => match ident.lit {
                        syn::Lit::Str(ref text) => items.handlers.push(text.value()),
                        _ => panic!("Attribute guard expects literal string!"),
                    },
                    attr => panic!("Unknown attribute key is specified: {}. Allowed: guard, hook, handler", attr)
                },
                attr => panic!("Unknown attribute{:?}", attr)
            }
        }

        let path = path.expect("Scope's path is not specified!");

        Self {
            ast,
            name,
            path,
            items,
            scope_items,
        }
    }

    pub fn generate(&self) -> TokenStream {
        let text = self.to_string();

        match text.parse() {
            Ok(res) => res,
            Err(error) => panic!("Error: {:?}\nGenerated code: {}", error, text)
        }
    }

}

impl fmt::Display for Args {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ast = &self.ast;

        let module_name = format!("{}_scope", self.name);
        let module_name = syn::Ident::new(&module_name, ast.ident.span());
        let ast = match ast.expr.as_ref() {
            syn::Expr::Block(expr) => quote!(pub mod #module_name #expr),
            _ => panic!("Unexpect non-block ast in scope macro")
        };

        writeln!(f, "{}\n", ast)?;
        writeln!(f, "#[allow(non_camel_case_types)]")?;
        writeln!(f, "struct {};\n", self.name)?;
        writeln!(f, "impl<P: 'static> actix_web::dev::HttpServiceFactory<P> for {} {{", self.name)?;
        writeln!(f, "    fn register(self, config: &mut actix_web::dev::ServiceConfig<P>) {{")?;
        write!(f, "        let scope = actix_web::Scope::new(\"{}\")", self.path)?;

        for hook in self.items.hooks.iter() {
            write!(f, ";\n        let scope = {}(scope)", hook)?;
        }
        for hook in self.scope_items.hooks.iter() {
            write!(f, ";\n        let scope = {}::{}(scope)", module_name, hook)?;
        }

        for guard in self.items.guards.iter() {
            write!(f, ".guard(actix_web::guard::fn_guard({}))", guard)?;
        }
        for guard in self.scope_items.guards.iter() {
            write!(f, ".guard(actix_web::guard::fn_guard({}::{}))", module_name, guard)?;
        }

        for handler in self.items.handlers.iter() {
            write!(f, ".service({})", handler)?;
        }
        for handler in self.scope_items.handlers.iter() {
            write!(f, ".service({}::{})", module_name, handler)?;
        }

        if let Some(default) = self.scope_items.default.as_ref() {
            write!(f, ".default_resource({}::{})", module_name, default)?;
        }

        writeln!(f, ";\n")?;
        writeln!(f, "        actix_web::dev::HttpServiceFactory::register(scope, config)")?;
        writeln!(f, "    }}\n}}")
    }
}

pub struct ImplScope {
    ast: syn::ItemImpl,
    name: String,
    scope_items: ScopeItems,
}

impl ImplScope {
    pub fn new(input: TokenStream) -> Self {
        let mut ast: syn::ItemImpl = syn::parse(input).expect("Parse input as impl block");

        let name = match &*ast.self_ty {
            &syn::Type::Path(ref type_path) => {
                if type_path.path.segments.len() > 1 {
                    panic!("Too many type params in path!");
                }

                let item = type_path.path.segments.first().expect("To have at least one path segment in Self type");
                item.value().ident.to_string()
            },
            _ => panic!("Scope can be implemented only for impl Block")
        };

        let mut scope_items: Vec<syn::Item> = Vec::new();

        let mut used_attrs = HashSet::new();
        used_attrs.insert("get");
        used_attrs.insert("put");
        used_attrs.insert("delete");
        used_attrs.insert("handler");
        used_attrs.insert("hook");
        used_attrs.insert("guard");

        for item in ast.items.iter_mut() {
            match item {
                syn::ImplItem::Method(method) => {
                    let item = syn::ItemFn {
                        attrs: method.attrs.clone(),
                        vis: method.vis.clone(),
                        constness: method.sig.constness.clone(),
                        unsafety: method.sig.unsafety.clone(),
                        asyncness: method.sig.asyncness.clone(),
                        abi: method.sig.abi.clone(),
                        ident: method.sig.ident.clone(),
                        decl: Box::new(method.sig.decl.clone()),
                        block: Box::new(method.block.clone()),
                    };

                    scope_items.push(syn::Item::Fn(item));
                    method.attrs.retain(|attr| {
                        for bound in attr.path.segments.iter() {
                            if used_attrs.contains(bound.ident.to_string().as_str()) {
                                return false;
                            }
                        }

                        true
                    })
                },
                _ => continue
            }
        }

        let scope_items = ScopeItems::from_items(&scope_items);

        Self {
            ast,
            name,
            scope_items,
        }
    }

    pub fn generate(&self) -> TokenStream {
        let text = self.to_string();

        match text.parse() {
            Ok(res) => res,
            Err(error) => panic!("Error: {:?}\nGenerated code: {}", error, text)
        }
    }
}

impl fmt::Display for ImplScope {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ast = &self.ast;
        writeln!(f, "{}\n", quote!(#ast))?;

        writeln!(f, "impl<P: 'static> actix_web::dev::HttpServiceFactory<P> for {} {{", self.name)?;
        writeln!(f, "    fn register(self, config: &mut actix_web::dev::ServiceConfig<P>) {{")?;

        write!(f, "        let scope = self.actix_scope()")?;

        for hook in self.scope_items.hooks.iter() {
            write!(f, ";\n        let scope = Self::{}(scope)", hook)?;
        }

        for guard in self.scope_items.guards.iter() {
            write!(f, ".guard(actix_web::guard::fn_guard(Self::{}))", guard)?;
        }

        for fun in self.scope_items.raw_routes.iter() {
            let mut guard = crate::route::GuardType::None;
            let mut attrs = Vec::new();

            for attr in fun.attrs.iter() {
                let is_found = if attr.path.is_ident("get") {
                    guard = crate::route::GuardType::Get;
                    true
                } else if attr.path.is_ident("put") {
                    guard = crate::route::GuardType::Put;
                    true
                } else if attr.path.is_ident("post") {
                    guard = crate::route::GuardType::Post;
                    true
                } else if attr.path.is_ident("delete") {
                    guard = crate::route::GuardType::Delete;
                    true
                } else if attr.path.is_ident("hanlder") {
                    true
                } else {
                    false
                };

                if is_found {
                    let args: proc_macro::TokenStream = attr.tts.clone().into();
                    let args = match args.into_iter().next().expect("To have single token tree in route's args") {
                        proc_macro::TokenTree::Group(group) => group.stream(),
                        args => panic!("Expected route's args as TokenTree's Group, but got {:?}", args),
                    };
                    //That's hidden API, dangerous?
                    let mut args = syn::parse_macro_input::parse::<syn::AttributeArgs>(args).expect("To parse route's arguments");
                    mem::swap(&mut args, &mut attrs);
                    break;
                }
            }

            let (path, resource_type, extra_guards) = crate::route::parse_meta_attrs(&fun, &attrs);
            let path = path.expect("Route's handle misses path");
            write!(f, ".route(\"{}\", actix_web::Route::new()", path)?;

            if guard != GuardType::None {
                write!(f, ".guard(actix_web::guard::{}())", guard)?;
            }

            for guard in extra_guards.iter() {
                write!(f, ".guard(self.{})", guard)?;
            }

            writeln!(f, ".{}(Self::{}))", resource_type, fun.ident)?;
        }

        if let Some(default) = self.scope_items.default.as_ref() {
            write!(f, ".default_resource(Self::{})", default)?;
        }

        writeln!(f, ";\n")?;
        writeln!(f, "        actix_web::dev::HttpServiceFactory::register(scope, config)")?;

        writeln!(f, "    }}\n}}")
    }
}
