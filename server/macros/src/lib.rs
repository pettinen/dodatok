use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

mod alert_enum;
mod sql_enum;
mod test_with_client;

use alert_enum::AlertEnum;
use sql_enum::SqlEnum;
use test_with_client::TestWithClient;

#[proc_macro_attribute]
pub fn alert_enum(_meta: TokenStream, input: TokenStream) -> TokenStream {
    parse_macro_input!(input as AlertEnum)
        .to_token_stream()
        .into()
}

#[proc_macro_attribute]
pub fn sql_enum(_meta: TokenStream, input: TokenStream) -> TokenStream {
    parse_macro_input!(input as SqlEnum)
        .to_token_stream()
        .into()
}

#[proc_macro_attribute]
pub fn test_with_client(_meta: TokenStream, input: TokenStream) -> TokenStream {
    parse_macro_input!(input as TestWithClient)
        .to_token_stream()
        .into()
}
