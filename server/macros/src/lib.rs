use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, NestedMeta};

mod alert_enum;
mod sql_enum;
mod test_with_client;

use alert_enum::AlertEnum;
use sql_enum::SqlEnum;
use test_with_client::TestWithClient;

#[proc_macro_attribute]
pub fn alert_enum(attrs: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attrs as syn::AttributeArgs);
    let is_response_error = match &attrs[..] {
        [] => false,
        [NestedMeta::Meta(meta)] if meta.path().is_ident("response_error") => {
            true
        },
        _ => panic!("invalid use of #[alert_enum]"),
    };
    let mut input = parse_macro_input!(input as AlertEnum);
    input.set_response_error(is_response_error);
    input.to_token_stream().into()
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
