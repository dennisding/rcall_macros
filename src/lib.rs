use proc_macro::TokenStream;
use proc_macro2;

use syn;
use quote::{self};

use std::collections::HashSet;

/// #[rcall::protocol]
/// triat ServerProtocol {
///     #[rcall::rpc(1)]
///     fn hello_from_client(&self, msg: &str);
///     #[rcall::rcp(2)]
///     fn login(&self, name: &str, password: &str);
/// }

#[proc_macro_attribute]
pub fn rpc(_input_item: TokenStream, annotated_item: TokenStream) -> TokenStream {
    annotated_item
}

struct FunInfo<'a> {
    name: &'a syn::Ident,
    id: i32,
    args: Vec<(&'a syn::Ident, &'a syn::Type)>
}

impl<'a> FunInfo<'a> {
    pub fn new(name: &'a syn::Ident, id: i32, args: Vec<(&'a syn::Ident, &'a syn::Type)>) -> Self {
        FunInfo {
            name,
            id,
            args
        }
    }
}

fn parse_rpc_id(item: &syn::TraitItemFn) -> i32 {
    let mut result: i32 = 0;
    for attr in item.attrs.iter() {
        if let syn::Meta::List(list_attr) = &attr.meta {
            let int_result = syn::parse2::<syn::LitInt>(list_attr.tokens.clone());
            if let Ok(int_expr) = int_result {
                if let Ok(int_value) = int_expr.base10_parse::<i32>() {
                    result = int_value;
                }
            }
        }
    }

    if result != 0 {
        if result < 10 {
            panic!("invalid rpc_id: {}", result);
        }
    }

    return result;
}

fn parse_fun_args(item: &syn::TraitItemFn) -> Vec<(&syn::Ident, &syn::Type)> {
    let mut args = Vec::new();
    // fn hello(&self, name: &str)
    // skip the &self
    for arg in item.sig.inputs.iter().skip(1) {
        if let syn::FnArg::Typed(syn::PatType{ pat, ty, ..}) = arg {
            if let syn::Pat::Ident(ident) = &**pat {
                let info = (&ident.ident, &**ty); 
                args.push(info);
            }
        }
    }

    args
}

fn parse_fun_item(item: &syn::TraitItemFn) -> FunInfo {
    let id = parse_rpc_id(item);
    let args = parse_fun_args(item);
    let info = FunInfo::new(&item.sig.ident, id, args);

    return info;
}

fn setup_rpc_id(fun_infos: &mut Vec<FunInfo>) {
    let mut ids: HashSet<i32> = HashSet::new();

    // gather ids
    for fun_info in fun_infos.iter() {
        if fun_info.id != 0 {
            ids.insert(fun_info.id);
        }
    }
    // gen new id for default rpc
    let mut last_index = 10;
    for fun_info in fun_infos.iter_mut() {
        if fun_info.id == 0 {
            let new_index = loop {
                last_index = last_index + 1;
                if ids.insert(last_index) {
                    break last_index;
                } else {
                    continue;
                }
            };
            fun_info.id = new_index;
        }
    }
}

fn gen_arg_names<'a>(fun_info: &'a FunInfo) -> Vec<&'a syn::Ident> {
    let mut result = Vec::new();
    for arg in fun_info.args.iter() {
        result.push(arg.0);
    }

    result
}

fn gen_arg_types<'a>(fun_info: &'a FunInfo) -> Vec<&'a syn::Type> {
    let mut result = Vec::new();
    for arg in fun_info.args.iter() {
        result.push(arg.1);
    }

    result
}

fn gen_match_expr(fun_infos: &Vec<FunInfo>) -> Vec<proc_macro2::TokenStream>{
    let mut match_exprs = Vec::new();

    for fun_info in fun_infos.iter() {
        let id = fun_info.id;
        let name = fun_info.name;
        let arg_names = gen_arg_names(&fun_info);
        let arg_types = gen_arg_types(&fun_info);

        let tokens = quote::quote!{
            #id => {
                if let Some((#(#arg_names),* )) = rcall::unpack!(packet, #(#arg_types),* ) {
                    self.#name(#(#arg_names),*);
                } else {
                    println!("error in calling rpc: {}:{}", rpc_id, stringify!(#name));
                }
            }
        };
        match_exprs.push(tokens);
    }

    match_exprs
}

fn gen_remote_fun(fun_infos: &Vec<FunInfo>) -> Vec<proc_macro2::TokenStream> {
    let mut remote_funs = Vec::new();

    for fun_info in fun_infos.iter() {
        let rpc_id = fun_info.id;
        let name = fun_info.name;
        let arg_names = gen_arg_names(&fun_info);
        let arg_types = gen_arg_types(&fun_info);

    // fn hello_from_server(&mut self, msg: String) {
    //     let rpc_id: rcall::RpcId = 1;
    //     let packet = rcall::pack!(rpc_id, msg);
    //     self.sender.send(packet);
    // }

        let pack_fun = quote::quote! {
            pub fn #name(&mut self, #(#arg_names: #arg_types),*) {
                let rpc_id: rcall::RpcId = #rpc_id;
                let packet = rcall::pack!(rpc_id, #(#arg_names),*);
                self.sender.send(packet);
            }
        };
        remote_funs.push(pack_fun);
    }

    remote_funs
}

#[proc_macro_attribute]
pub fn protocol(_input_item: TokenStream, annotated_item: TokenStream) -> TokenStream {
    let mut trait_infos = syn::parse_macro_input!(annotated_item as syn::ItemTrait);

    let mut fun_infos: Vec<FunInfo> = Vec::new();
    for item in trait_infos.items.iter() {
        if let syn::TraitItem::Fn(fun_item) = item {
            fun_infos.push(parse_fun_item(fun_item));
        }
    }

    // gen token stream

    setup_rpc_id(&mut fun_infos);

//    let ident = trait_infos.ident.clone();
    let ident = trait_infos.ident.clone();
    // xxx_Remote
    let remote_ident = syn::Ident::new(&format!("{}_Remote", ident), proc_macro2::Span::call_site()); // xxx_Remote
    let match_expr = gen_match_expr(&fun_infos);
    let remote_fun = gen_remote_fun(&fun_infos);

    let dispatcher = quote::quote! {
        fn _dispatch_rpc(&mut self, rpc_id: i32, mut packet: rcall::packer::Packet) {
            match rpc_id {
                #(#match_expr)*
                _ => {
                    println!("invalid rpc_id: {}", rpc_id);
                }
            }
        }
    }.into();

    let dispatch_item = syn::parse_macro_input!(dispatcher as syn::TraitItem);
    trait_infos.items.push(dispatch_item);

    let expanded = quote::quote! {
        #trait_infos

        pub struct #remote_ident<T: rcall::Sender> {
            sender: T
        }

        impl<T: rcall::Sender> #remote_ident<T> {
            pub fn new(sender: T) -> Self {
                #remote_ident {
                    sender
                }
            }

            pub fn close(&mut self) {
                self.sender.close();
            }

            // expand remote fun
            #(#remote_fun)*
        }
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn protocol_impl(_input_item: TokenStream, annotated_item: TokenStream) -> TokenStream  {

    annotated_item
}

#[proc_macro_derive(Dispatcher)]
pub fn services_derive(input: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(input as syn::ItemStruct);
    let ident = item.ident;

    let generic = item.generics;
    let (impl_generic, type_generic, where_clause) = generic.split_for_impl();

    let code = quote::quote! {
        impl #impl_generic rcall::RpcDispatcher for #ident #type_generic #where_clause {
            fn dispatch_rpc(&mut self, mut packet: rcall::Packet) {
                use rcall::UnpackFrom;
                if let Some(rpc_id) = <rcall::RpcId>::unpack_from(&mut packet) {
                    self._dispatch_rpc(rpc_id, packet);
                }
                else {
                    println!("invalid rpc_id!");
                }
//                self._dispatch_rpc(rpc_id, packet);
            }
        }
    };

    code.into()
}

// protocols::ImplInServer_Remote<rcall::ClientSender>;
#[proc_macro]
pub fn client_to_remote_type(input: TokenStream) -> TokenStream {
    let code_string = input.to_string() + "_Remote<rcall::ClientSender>";
    let token_stream: proc_macro2::TokenStream = syn::parse_str(&code_string).expect("parse_error");

    let tokens = quote::quote! {
        #token_stream
    };

    tokens.into()
}

/// generate a server remote type
/// type Remote = rcall::server_to_remote_type(ImplInClientProtocol)
/// let remote = Remote::new(sender)
#[proc_macro]
pub fn server_to_remote_type(input: TokenStream) -> TokenStream {
    let code_string = input.to_string() + "_Remote<rcall::ServerSender>";
    let token_stream: proc_macro2::TokenStream = syn::parse_str(&code_string).expect("parse_error");

    let tokens = quote::quote! {
        #token_stream
    };

    tokens.into()
}
