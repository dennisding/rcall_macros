use proc_macro::TokenStream;
use proc_macro2;

use syn;
use quote;

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
//     let input = syn::parse_macro_input!(input_item as syn::LitInt);
// //    let function = syn::parse_macro_input!(annotated_item as syn::TraitItemFn);
//     let expanded = quote::quote! {
//         #input
//     };

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
        let _ = attr.parse_nested_meta(|meta| {
            let expr: syn::Expr = meta.value()?.parse()?;
            if let syn::Expr::Lit(expr_lit) = expr {
                if let syn::Lit::Int(lit_int) = expr_lit.lit {
                    if let Ok(int_value) = lit_int.base10_parse() {
                        result = int_value;
                    }
                }
            }
            return Ok(());
        });
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

//     // parse attribute
//     for attr in item.attrs.iter() {
// //        attr.par
//         let _ = attr.parse_nested_meta(|meta| {
//             let expr: syn::Expr = meta.value()?.parse()?;
//             if let syn::Expr::Lit(expr_lit) = expr {
//                 if let syn::Lit::Int(lit_int) = expr_lit.lit {
//                     println!("hello: {}", lit_int);
//                 }
//             }
//             return Ok(());
//             // match expr {
//             //     syn::Expr::Lit(expr_lit) => {
//             //         match expr_lit.lit {
//             //             syn::Lit::Int(lit_int) => {
//             //                 println!("hello: {}", lit_int);
//             //                 lit_int.base10_parse();
//             //             }
//             //             _ => {}
//             //         }
//             //     }

//             //     _ => {

//             //     }
//             // }
//             // return Ok(());
//         });
//     }

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
                    self.#name(#(#arg_names),*).await;
                }
            }
        };
        match_exprs.push(tokens);
    }

    match_exprs
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
    let match_expr = gen_match_expr(&fun_infos);

    let dispatcher = quote::quote! {
        async fn _dispatch_rpc(&mut self, rpc_id: i32, mut packet: crate::packer::Packet) {
            match rpc_id {
                #(#match_expr)*
                _ => {

                }
            }
        }
    }.into();

    let dispatch_item = syn::parse_macro_input!(dispatcher as syn::TraitItem);
    trait_infos.items.push(dispatch_item);

    let expanded = quote::quote! {
        #trait_infos

//        impl<T: Server> crate::network::RpcDispatcher for T {
        // impl<T: #ident> crate::network::RpcDispatcher for T {
        //     async fn dispatch_rpc(&mut self, rpc_id: i32, mut packet: packer::Packet) {
        //         match rpc_id {
        //             #(#match_expr)*
        //             _ => {

        //             }
        //         }
        //     }
        // }
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn protocol_impl(_input_item: TokenStream, annotated_item: TokenStream) -> TokenStream  {

    annotated_item
}

#[proc_macro_derive(Protocol)]
pub fn protocol_derive(input: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(input as syn::ItemStruct);
    let ident = item.ident;

    let code = quote::quote! {
        impl crate::network::RpcDispatcher for #ident {
            async fn dispatch_rpc(&mut self, rpc_id: i32, packet: crate::packer::Packet) {
                self._dispatch_rpc(rpc_id, packet).await;
            }
        }
    };
    // // 将 Rust 代码解析为语法树以便进行操作
    // let ast = syn::parse(input).unwrap();

    // // 构建 trait 实现
    // ast
    code.into()
//    TokenStream::new()
}

// fn main() {
//     println!("Hello, world!");
// }
