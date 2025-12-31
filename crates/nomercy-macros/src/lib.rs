use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item};

fn passthrough(item: Item) -> TokenStream {
    quote!(#item).into()
}

#[proc_macro_attribute]
pub fn system(_args: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as Item);
    passthrough(item)
}

#[proc_macro_attribute]
pub fn op(_args: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as Item);
    passthrough(item)
}

#[proc_macro_attribute]
pub fn invariant(_args: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as Item);
    passthrough(item)
}

#[proc_macro_attribute]
pub fn observe(_args: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as Item);
    passthrough(item)
}
