#![recursion_limit = "512"]

extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::{
    braced,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote, parse_str,
    spanned::Spanned,
    token::Comma,
    Attribute, FnArg, Ident, ImplItem, ImplItemMethod, ImplItemType, ItemImpl, Pat, PatType,
    ReturnType, Token, Type, Visibility,
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
        // input.parse::<Token![async]>()?;
        input.parse::<Token![fn]>()?;
        let ident = input.parse()?;
        let content;
        parenthesized!(content in input);
        let mut args = Vec::new();
        let mut errors = Ok(());
        for arg in content.parse_terminated::<FnArg, Comma>(FnArg::parse)? {
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
                FnArg::Receiver(_) => {
                    extend_errors!(
                        errors,
                        syn::Error::new(arg.span(), "method args cannot start with self")
                    );
                }
            }
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

    let macro_args = parse_macro_input!(attr as syn::AttributeArgs);
    if !macro_args.is_empty() {
        return syn::Error::new(
            macro_args[0].span(),
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
        server_ident: &format_ident!("Serve{}", ident),
        client_ident: &format_ident!("{}Client", ident),
        request_ident: &format_ident!("{}Request", ident),
        response_ident: &format_ident!("{}Response", ident),
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
        future_types: &camel_case_fn_names
            .iter()
            .map(|name| parse_str(&format!("{name}Fut")).unwrap())
            .collect::<Vec<_>>(),
        derive_serialize: &derive_serialize,
    };

    let code = generator.into_token_stream().into();
    eprintln!("{:#}", code);
    code
}

/// generate an identifier consisting of the method name to CamelCase with
/// Fut appended to it.
fn associated_type_for_rpc(method: &ImplItemMethod) -> String {
    use heck::ToUpperCamelCase;
    method.sig.ident.unraw().to_string().to_upper_camel_case() + "Fut"
}

/// Transforms an async function into a sync one, returning a type declaration
/// for the return type (a future).
fn transform_method(method: &mut ImplItemMethod) -> ImplItemType {
    method.sig.asyncness = None;

    // get either the return type or ().
    let ret = match &method.sig.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, ret) => quote!(#ret),
    };

    let fut_name = associated_type_for_rpc(method);
    let fut_name_ident = Ident::new(&fut_name, method.sig.ident.span());

    // generate the updated return signature.
    method.sig.output = parse_quote! {
        -> ::core::pin::Pin<Box<
                dyn ::core::future::Future<Output = #ret> + ::core::marker::Send
            >>
    };

    // transform the body of the method into Box::pin(async move { body }).
    let block = method.block.clone();
    method.block = parse_quote! [{
        Box::pin(async move
            #block
        )
    }];

    // generate and return type declaration for return type.
    let t: ImplItemType = parse_quote! {
        type #fut_name_ident = ::core::pin::Pin<Box<dyn ::core::future::Future<Output = #ret> + ::core::marker::Send>>;
    };

    t
}

#[proc_macro_attribute]
pub fn server(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(input as ItemImpl);
    let span = item.span();

    // the generated type declarations
    let mut types: Vec<ImplItemType> = Vec::new();
    let mut expected_non_async_types: Vec<(&ImplItemMethod, String)> = Vec::new();
    let mut found_non_async_types: Vec<&ImplItemType> = Vec::new();

    for inner in &mut item.items {
        match inner {
            ImplItem::Method(method) => {
                if method.sig.asyncness.is_some() {
                    // if this function is declared async, transform it into a regular function
                    let type_decl = transform_method(method);
                    types.push(type_decl);
                } else {
                    // If it's not async, keep track of all required associated types for better
                    // error reporting.
                    expected_non_async_types.push((method, associated_type_for_rpc(method)));
                }
            }
            ImplItem::Type(type_decl) => found_non_async_types.push(type_decl),
            _ => {}
        }
    }

    if let Err(e) =
        verify_types_were_provided(span, &expected_non_async_types, &found_non_async_types)
    {
        return TokenStream::from(e.to_compile_error());
    }

    // add the type declarations into the impl block
    for t in types.into_iter() {
        item.items.push(syn::ImplItem::Type(t));
    }

    TokenStream::from(quote!(#item))
}

fn verify_types_were_provided(
    span: Span,
    expected: &[(&ImplItemMethod, String)],
    provided: &[&ImplItemType],
) -> syn::Result<()> {
    let mut result = Ok(());
    for (method, expected) in expected {
        if !provided.iter().any(|type_decl| type_decl.ident == expected) {
            let mut e = syn::Error::new(
                span,
                format!("not all trait items implemented, missing: `{expected}`"),
            );
            let fn_span = method.sig.fn_token.span();
            e.extend(syn::Error::new(
                fn_span.join(method.sig.ident.span()).unwrap_or(fn_span),
                format!(
                    "hint: `#[may_rpc::server]` only rewrites async fns, and `fn {}` is not async",
                    method.sig.ident
                ),
            ));
            match result {
                Ok(_) => result = Err(e),
                Err(ref mut error) => error.extend(Some(e)),
            }
        }
    }
    result
}

// Things needed to generate the service items: trait, serve impl, request/response enums, and
// the client stub.
struct ServiceGenerator<'a> {
    service_ident: &'a Ident,
    server_ident: &'a Ident,
    client_ident: &'a Ident,
    request_ident: &'a Ident,
    response_ident: &'a Ident,
    vis: &'a Visibility,
    attrs: &'a [Attribute],
    rpcs: &'a [RpcMethod],
    camel_case_idents: &'a [Ident],
    future_types: &'a [Type],
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
                    fn #ident(#( #args ),*) -> #output;
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

    fn struct_server(&self) -> TokenStream2 {
        let &Self {
            vis, server_ident, ..
        } = self;

        quote! {
            /// A serving function to use with [may_rpc::server::InFlightRequest::execute].
            #[derive(Clone)]
            #vis struct #server_ident<S> {
                service: S,
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
            #vis trait #dispatch_service_indent: #service_ident
            {
                fn dispatch_req(req: #request_ident, rsp: &mut conetty::RspBuf) -> Result<(), may_rpc::conetty::WireError> {
                    match req {
                        #(
                            #request_ident::#camel_case_idents{ #( #arg_pats ),* } => {
                                may_rpc::bincode::serialize_into(rsp, &Self::#method_idents(#( #arg_pats ),*))
                                    .map_err(|e| may_rpc::conetty::WireError::ServerSerialize(e.to_string()))
                            }
                        )*
                    }
                }
            }

            impl<T: #service_ident> #dispatch_service_indent for T {}
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

    fn enum_response(&self) -> TokenStream2 {
        let &Self {
            derive_serialize,
            vis,
            response_ident,
            camel_case_idents,
            return_types,
            ..
        } = self;

        quote! {
            /// The response sent over the wire from the server to the client.
            #[allow(missing_docs)]
            #[derive(Debug)]
            #derive_serialize
            #vis enum #response_ident {
                #( #camel_case_idents(#return_types) ),*
            }
        }
    }

    // fn enum_response_future(&self) -> TokenStream2 {
    //     let &Self {
    //         vis,
    //         service_ident,
    //         response_fut_ident,
    //         camel_case_idents,
    //         future_types,
    //         ..
    //     } = self;

    //     quote! {
    //         /// A future resolving to a server response.
    //         #[allow(missing_docs)]
    //         #vis enum #response_fut_ident<S: #service_ident> {
    //             #( #camel_case_idents(<S as #service_ident>::#future_types) ),*
    //         }
    //     }
    // }

    // fn impl_debug_for_response_future(&self) -> TokenStream2 {
    //     let &Self {
    //         service_ident,
    //         response_fut_ident,
    //         // response_fut_name,
    //         ..
    //     } = self;

    //     quote! {
    //         impl<S: #service_ident> std::fmt::Debug for #response_fut_ident<S> {
    //             fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
    //                 fmt.debug_struct(#response_fut_name).finish()
    //             }
    //         }
    //     }
    // }

    // fn impl_future_for_response_future(&self) -> TokenStream2 {
    //     let &Self {
    //         service_ident,
    //         response_fut_ident,
    //         response_ident,
    //         camel_case_idents,
    //         ..
    //     } = self;

    //     quote! {
    //         impl<S: #service_ident> std::future::Future for #response_fut_ident<S> {
    //             type Output = #response_ident;

    //             fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>)
    //                 -> std::task::Poll<#response_ident>
    //             {
    //                 unsafe {
    //                     match std::pin::Pin::get_unchecked_mut(self) {
    //                         #(
    //                             #response_fut_ident::#camel_case_idents(resp) =>
    //                                 std::pin::Pin::new_unchecked(resp)
    //                                     .poll(cx)
    //                                     .map(#response_ident::#camel_case_idents),
    //                         )*
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

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
