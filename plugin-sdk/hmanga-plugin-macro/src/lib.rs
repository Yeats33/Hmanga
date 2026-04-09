use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn hmanga_plugin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
