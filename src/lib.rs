use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn hello_macro(input_item: TokenStream, annotated_item: TokenStream) -> TokenStream {
    annotated_item
}

// fn main() {
//     println!("Hello, world!");
// }
