use proc_macro::TokenStream;

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

#[proc_macro_attribute]
pub fn protocol(_input_item: TokenStream, annotated_item: TokenStream) -> TokenStream {
    let trait_infos = syn::parse_macro_input!(annotated_item as syn::ItemTrait);

    let mut fun_infos: Vec<FunInfo> = Vec::new();
    for item in trait_infos.items.iter() {
        if let syn::TraitItem::Fn(fun_item) = item {
            fun_infos.push(parse_fun_item(fun_item));
        }
    }

    // gen token stream

    // setup_rpc_id(&mut fun_infos);

    let expanded = quote::quote! {
        #trait_infos
    };

    expanded.into()
}

// fn main() {
//     println!("Hello, world!");
// }
