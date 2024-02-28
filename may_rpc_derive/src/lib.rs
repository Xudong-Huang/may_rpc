#![recursion_limit = "512"]

extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{
    braced,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    spanned::Spanned,
    Attribute, FnArg, Ident, Pat, PatType, ReturnType, Token, Type, Visibility,
};

/// Accumulates multiple errors into a result.
/// Only use this for recoverable errors, i.e. non-parse errors. Fatal errors should early exit to
/// avoid further complications.
macro_rules! extend_errors {
    ($errors: ident, $e: expr) => {
        match $errors {
            Ok(_) => $errors = Err($e),
            Err(ref mut errors) => errors.extend($e),
        }
    };
}

#[allow(unused_macros)]
macro_rules! d {
    ($v: expr) => {{
        eprintln!("{} = {:#?}", stringify!($v), $v);
    }};
}

struct Service {
    attrs: Vec<Attribute>,
    vis: Visibility,
    ident: Ident,
    rpcs: Vec<RpcMethod>,
}

struct RpcMethod {
    attrs: Vec<Attribute>,
    ident: Ident,
    args: Vec<PatType>,
    output: ReturnType,
}

impl Parse for Service {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        input.parse::<Token![trait]>()?;
        let ident: Ident = input.parse()?;
        let content;
        braced!(content in input);
        let mut rpcs = Vec::<RpcMethod>::new();
        while !content.is_empty() {
            rpcs.push(content.parse()?);
        }
        let mut ident_errors = Ok(());
        for rpc in &rpcs {
            if rpc.ident == "new" {
                extend_errors!(
                    ident_errors,
                    syn::Error::new(
                        rpc.ident.span(),
                        format!(
                            "method name conflicts with generated fn `{}Client::new`",
                            ident.unraw()
                        )
                    )
                );
            }
        }
        ident_errors?;

        Ok(Self {
            attrs,
            vis,
            ident,
            rpcs,
        })
    }
}

impl Parse for RpcMethod {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        input.parse::<Token![fn]>()?;
        let ident = input.parse()?;
        let content;
        parenthesized!(content in input);
        let mut args = Vec::new();
        let mut errors = Ok(());
        let mut found_self = false;
        for arg in content.parse_terminated(FnArg::parse, Token![,])? {
            match arg {
                FnArg::Typed(captured) if matches!(&*captured.pat, Pat::Ident(_)) => {
                    args.push(captured);
                }
                FnArg::Typed(captured) => {
                    extend_errors!(
                        errors,
                        syn::Error::new(captured.pat.span(), "patterns aren't allowed in RPC args")
                    );
                }
                FnArg::Receiver(me) => {
                    found_self = true;
                    if me.mutability.is_some() {
                        extend_errors!(errors, syn::Error::new(me.span(), "self can't be mutable"));
                    }
                    if me.reference.is_none() {
                        extend_errors!(errors, syn::Error::new(me.span(), "self must be &self"));
                    }
                }
            }
        }
        if !found_self {
            extend_errors!(
                errors,
                syn::Error::new(content.span(), "rpc method must start with &self")
            );
        }
        errors?;
        let output = input.parse()?;
        input.parse::<Token![;]>()?;

        Ok(Self {
            attrs,
            ident,
            args,
            output,
        })
    }
}

/// A helper attribute to avoid a direct dependency on Serde.
///
/// Adds the following annotations to the annotated item:
///
/// ```rust
/// #[derive(may_rpc::serde::Serialize, may_rpc::serde::Deserialize)]
/// #[serde(crate = "may_rpc::serde")]
/// # struct Foo;
/// ```
#[proc_macro_attribute]
pub fn derive_serde(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut gen: proc_macro2::TokenStream = quote! {
        #[derive(may_rpc::serde::Serialize, may_rpc::serde::Deserialize)]
        #[serde(crate = "may_rpc::serde")]
    };
    gen.extend(proc_macro2::TokenStream::from(item));
    proc_macro::TokenStream::from(gen)
}

/// Generates:
/// - service trait
/// - client stub struct
/// - dispatch service trait
/// - Request enums
#[proc_macro_attribute]
pub fn service(attr: TokenStream, input: TokenStream) -> TokenStream {
    use heck::ToUpperCamelCase;

    if !attr.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "may_rpc::service does not support this attr item",
        )
        .to_compile_error()
        .into();
    }

    let unit_type: &Type = &parse_quote!(());
    let Service {
        ref attrs,
        ref vis,
        ref ident,
        ref rpcs,
    } = parse_macro_input!(input as Service);

    let camel_case_fn_names: &Vec<_> = &rpcs
        .iter()
        .map(|rpc| rpc.ident.unraw().to_string().to_upper_camel_case())
        .collect();
    let args: &[&[PatType]] = &rpcs.iter().map(|rpc| &*rpc.args).collect::<Vec<_>>();
    let derive_serialize = {
        quote! {
            #[derive(may_rpc::serde::Serialize, may_rpc::serde::Deserialize)]
            #[serde(crate = "may_rpc::serde")]
        }
    };

    let methods = rpcs.iter().map(|rpc| &rpc.ident).collect::<Vec<_>>();

    let generator = ServiceGenerator {
        service_ident: ident,
        client_ident: &format_ident!("{}Client", ident),
        request_ident: &format_ident!("{}Request", ident),
        vis,
        args,
        method_attrs: &rpcs.iter().map(|rpc| &*rpc.attrs).collect::<Vec<_>>(),
        method_idents: &methods,
        attrs,
        rpcs,
        return_types: &rpcs
            .iter()
            .map(|rpc| match rpc.output {
                ReturnType::Type(_, ref ty) => ty,
                ReturnType::Default => unit_type,
            })
            .collect::<Vec<_>>(),
        arg_pats: &args
            .iter()
            .map(|args| args.iter().map(|arg| &*arg.pat).collect())
            .collect::<Vec<_>>(),
        camel_case_idents: &rpcs
            .iter()
            .zip(camel_case_fn_names.iter())
            .map(|(rpc, name)| Ident::new(name, rpc.ident.span()))
            .collect::<Vec<_>>(),
        derive_serialize: &derive_serialize,
    };
    let code = generator.into_token_stream();
    // eprintln!("{}", code);
    code.into()
}

// Things needed to generate the service items: trait, serve impl, request/response enums, and
// the client stub.
struct ServiceGenerator<'a> {
    service_ident: &'a Ident,
    client_ident: &'a Ident,
    request_ident: &'a Ident,
    vis: &'a Visibility,
    attrs: &'a [Attribute],
    rpcs: &'a [RpcMethod],
    camel_case_idents: &'a [Ident],
    method_idents: &'a [&'a Ident],
    method_attrs: &'a [&'a [Attribute]],
    args: &'a [&'a [PatType]],
    return_types: &'a [&'a Type],
    arg_pats: &'a [Vec<&'a Pat>],
    derive_serialize: &'a TokenStream2,
}

impl<'a> ServiceGenerator<'a> {
    fn trait_service(&self) -> TokenStream2 {
        let &Self {
            attrs,
            rpcs,
            vis,
            return_types,
            service_ident,
            ..
        } = self;

        let types_and_fns = rpcs.iter().zip(return_types.iter()).map(
            |(
                RpcMethod {
                    attrs, ident, args, ..
                },
                output,
            )| {
                quote! {
                    #( #attrs )*
                    fn #ident(&self, #( #args ),*) -> #output;
                }
            },
        );

        quote! {
            #( #attrs )*
            #vis trait #service_ident: Sized {
                #( #types_and_fns )*
            }
        }
    }

    fn impl_dispatch_for_server(&self) -> TokenStream2 {
        let &Self {
            request_ident,
            service_ident,
            camel_case_idents,
            arg_pats,
            method_idents,
            vis,
            ..
        } = self;

        let dispatch_service_indent = format_ident!("{}ServiceDispatch", service_ident);
        quote! {
            #vis trait #dispatch_service_indent: #service_ident + std::panic::RefUnwindSafe
            {
                fn dispatch_req(&self, req: #request_ident, rsp: &mut may_rpc::conetty::RspBuf) -> Result<(), may_rpc::conetty::WireError> {
                    match req {
                        #(
                            #request_ident::#camel_case_idents{ #( #arg_pats ),* } => match std::panic::catch_unwind(|| self.#method_idents(#( #arg_pats ),*)) {
                                Ok(ret) => may_rpc::bincode::serialize_into(rsp, &ret).map_err(|e| may_rpc::conetty::WireError::ServerSerialize(e.to_string())),
                                Err(_) => Err(may_rpc::conetty::WireError::Status("rpc panicked in server!".to_owned())),
                            }
                        )*
                    }
                }
            }

            impl<T: #service_ident + std::panic::RefUnwindSafe> #dispatch_service_indent for T {}
        }
    }

    fn enum_request(&self) -> TokenStream2 {
        let &Self {
            derive_serialize,
            vis,
            request_ident,
            camel_case_idents,
            args,
            ..
        } = self;

        quote! {
            /// The request sent over the wire from the client to the server.
            #[allow(missing_docs)]
            #[derive(Debug)]
            #derive_serialize
            #vis enum #request_ident {
                #( #camel_case_idents{ #( #args ),* } ),*
            }
        }
    }

    fn struct_client(&self) -> TokenStream2 {
        let &Self {
            vis, client_ident, ..
        } = self;

        quote! {
            #[allow(unused)]
            #[derive(Debug)]
            /// The client stub that makes RPC calls to the server.
            #vis struct #client_ident<S: may_rpc::conetty::StreamExt>{
                transport: may_rpc::conetty::MultiplexClient<S>,
            }
        }
    }

    fn impl_client_new(&self) -> TokenStream2 {
        let &Self {
            client_ident, vis, ..
        } = self;

        quote! {
            impl<S: may_rpc::conetty::StreamExt> #client_ident<S> {
                /// Returns a new client stub that sends requests over the given transport.
                #vis fn new(stream: S) -> std::io::Result<Self> {
                    let transport = may_rpc::conetty::MultiplexClient::new(stream)?;
                    Ok(Self { transport })
                }

                /// set the read timeout value for the client
                #vis fn set_timeout(&mut self, timeout: std::time::Duration) {
                    self.transport.set_timeout(timeout);
                }
            }
        }
    }

    fn impl_client_rpc_methods(&self) -> TokenStream2 {
        let &Self {
            client_ident,
            request_ident,
            method_attrs,
            vis,
            method_idents,
            args,
            return_types,
            arg_pats,
            camel_case_idents,
            ..
        } = self;

        quote! {
            impl<S: may_rpc::conetty::StreamExt> #client_ident<S> {
                #(
                    #[allow(unused)]
                    #( #method_attrs )*
                    #vis fn #method_idents(&self, #( #args ),*) -> Result<#return_types, may_rpc::conetty::Error> {
                        use may_rpc::conetty::Client;
                        let mut req = may_rpc::conetty::ReqBuf::new();
                        // serialize the request
                        let request = #request_ident::#camel_case_idents { #( #arg_pats ),* };
                        may_rpc::bincode::serialize_into(&mut req, &request)
                            .map_err(|e| may_rpc::conetty::Error::ClientSerialize(e.to_string()))?;
                        // call the server
                        let rsp_frame = self.transport.call_service(req)?;
                        let rsp = rsp_frame.decode_rsp()?;
                        // deserialized the response
                        may_rpc::bincode::deserialize(rsp)
                            .map_err(|e| may_rpc::conetty::Error::ClientDeserialize(e.to_string()))
                    }
                )*
            }
        }
    }
}

impl<'a> ToTokens for ServiceGenerator<'a> {
    fn to_tokens(&self, output: &mut TokenStream2) {
        output.extend(vec![
            self.trait_service(),
            self.enum_request(),
            self.struct_client(),
            self.impl_client_new(),
            self.impl_client_rpc_methods(),
            self.impl_dispatch_for_server(),
        ])
    }
}

fn get_attr(attr_ident: &str, attrs: Vec<syn::Attribute>) -> Option<syn::Attribute> {
    attrs
        .into_iter()
        .find(|attr| attr.path().segments.len() == 1 && attr.path().segments[0].ident == attr_ident)
}

fn get_service_from_attr(attr: Option<syn::Attribute>) -> Result<syn::Path, syn::Error> {
    match attr {
        Some(a) => match a.meta {
            syn::Meta::List(l) => l.parse_args(),
            _ => Err(syn::Error::new(
                a.span(),
                "`service` attributes need at least one param",
            )),
        },
        None => Err(syn::Error::new(
            attr.span(),
            "expected `service` attributes",
        )),
    }
}

#[proc_macro_derive(Server, attributes(service))]
pub fn derive_rpc_server(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let struct_ident = ast.ident;
    let attrs = ast.attrs;

    let service_attr = get_attr("service", attrs);
    let service = get_service_from_attr(service_attr);

    let mut service = match service {
        Err(err) => return err.to_compile_error().into(),
        Ok(s) => s,
    };
    let mut service_request = service.clone();

    if let Some(seg) = service.segments.last_mut() {
        seg.ident = Ident::new(
            &format!("{}ServiceDispatch", seg.ident.to_token_stream()),
            seg.span(),
        );
    }

    if let Some(seg) = service_request.segments.last_mut() {
        seg.ident = Ident::new(
            &format!("{}Request", seg.ident.to_token_stream()),
            seg.span(),
        );
    }

    let out = quote!(
        impl may_rpc::conetty::Server for #struct_ident {
            fn service(&self, req: &[u8], rsp: &mut may_rpc::conetty::RspBuf) -> Result<(), may_rpc::conetty::WireError> {
                use #service;
                // deserialize the request
                let request: #service_request = may_rpc::bincode::deserialize(req)
                    .map_err(|e| may_rpc::conetty::WireError::ServerDeserialize(e.to_string()))?;
                // get the dispatch_fn
                self.dispatch_req(request, rsp)
            }
        }
    );
    // eprintln!("{}", out);
    out.into()
}
