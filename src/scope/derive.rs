use quote::quote;
use proc_macro::TokenStream;

use std::fmt;

pub struct Args {
    path: String,
    service: Vec<String>,
    guards: Vec<String>,
    ast: syn::DeriveInput,
}

impl Args {
    pub fn new(ast: syn::DeriveInput) -> Self {
        let struct_data = match ast.data {
            syn::Data::Struct(ref data) => data,
            _ => panic!("derive(Scope) is available for structs only"),
        };

        let mut path = "/".to_owned();
        for attr in ast.attrs.iter().flat_map(|attr| attr.parse_meta().ok()) {
            if attr.name() == "path" {
                match attr {
                    syn::Meta::NameValue(meta) => match meta.lit {
                        syn::Lit::Str(text) => {
                            path = text.value()
                        },
                        _ => panic!("'path' attribute is invalid, should string value"),
                    },
                    _ => panic!("'path' attribute is invalid, should contain value"),
                }
            }
        }

        let mut service = Vec::new();
        let mut guards = Vec::new();
        for field in struct_data.fields.iter() {
            for meta in field.attrs.iter().filter_map(|attr| attr.interpret_meta()) {
                let variable_name = field.ident.as_ref().expect("Named field is needed");

                if meta.name() == "service" {
                    match meta {
                        syn::Meta::Word(_) => {
                            service.push(variable_name.to_string())
                        },
                        _ => panic!("'service' attribute for field '{}' is invalid. Should have no value", variable_name)
                    }
                } else if meta.name() == "guard" {
                    match meta {
                        syn::Meta::Word(_) => {
                            guards.push(variable_name.to_string())
                        },
                        _ => panic!("'guard' attribute for field '{}' is invalid. Should have no value", variable_name)
                    }
                }
            }
        }

        Self {
            path,
            service,
            guards,
            ast
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
        let (impl_gen, type_gen, where_clause) = self.ast.generics.split_for_impl();

        writeln!(f, "{} {}{} {{", quote!(impl#impl_gen), self.ast.ident, quote!(#type_gen #where_clause))?;

        writeln!(f, "    pub fn actix_scope<P: 'static>(self) -> actix_web::Scope<P> {{")?;

        write!(f, "        actix_web::Scope::new(\"{}\")", self.path)?;

        for service in self.service.iter() {
            write!(f, ".service(self.{})", service)?;
        }
        for guard in self.guards.iter() {
            write!(f, ".guard(self.{})", guard)?;
        }

        writeln!(f, "\n    }}")?;
        writeln!(f, "}}")
    }
}

