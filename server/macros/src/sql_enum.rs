use convert_case::{Case, Casing};
use proc_macro::TokenStream as CompilerTokenStream;
use proc_macro2::{Ident, Span, TokenStream as ProcMacro2TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseBuffer},
    parse_macro_input, Attribute, ItemEnum, LitStr, Variant,
};

#[proc_macro_attribute]
pub fn sql_enum(_meta: CompilerTokenStream, input: CompilerTokenStream) -> CompilerTokenStream {
    parse_macro_input!(input as SqlEnum)
        .to_token_stream()
        .into()
}

#[derive(Debug)]
struct SqlEnum {
    enum_: ItemEnum,
    variants: Vec<SqlEnumVariant>,
}

impl Parse for SqlEnum {
    fn parse(input: &ParseBuffer) -> syn::Result<Self> {
        let enum_ = input.parse::<ItemEnum>()?;
        let variants = enum_
            .variants
            .clone()
            .into_iter()
            .map(|variant| SqlEnumVariant { variant })
            .collect();

        Ok(Self { enum_, variants })
    }
}

impl ToTokens for SqlEnum {
    fn to_tokens(&self, tokens: &mut ProcMacro2TokenStream) {
        let attrs = &self.enum_.attrs;
        let vis = &self.enum_.vis;
        let ident = &self.enum_.ident;
        let generics = &self.enum_.generics;
        let name_snake_case = to_snake_case(&self.enum_.ident);
        let variants = &self.variants;
        tokens.extend(quote! {
            #[derive(FromSql, Serialize)]
            #[postgres(name = #name_snake_case)]
            #(#attrs)*
            #vis enum #ident #generics {
                #(#variants),*
            }
        })
    }
}

#[derive(Debug)]
struct SqlEnumVariant {
    variant: Variant,
}

impl ToTokens for SqlEnumVariant {
    fn to_tokens(&self, tokens: &mut ProcMacro2TokenStream) {
        let attrs = &self.variant.attrs;
        let ident = &self.variant.ident;
        let fields = &self.variant.fields;

        let name_ident = Ident::new("name", Span::call_site());
        let (name_attrs, other_attrs): (Vec<&Attribute>, Vec<&Attribute>) = attrs
            .iter()
            .partition(|attr| attr.path.is_ident(&name_ident));
        if name_attrs.len() > 1 {
            panic!("multiple name(...) attributes specified for sql_enum variant");
        }
        let snake_case_name = if let Some(name_attr) = name_attrs.first() {
            name_attr.parse_args::<LitStr>().unwrap().value()
        } else {
            to_snake_case(ident)
        };

        tokens.extend(quote! {
            #[postgres(name = #snake_case_name)]
            #[serde(rename = #snake_case_name)]
            #(#other_attrs)*
            #ident #fields
        });
        if let Some(discriminant) = &self.variant.discriminant {
            let expr = &discriminant.1;
            tokens.extend(quote! {
                = #expr
            });
        }
    }
}

fn to_snake_case(ident: &Ident) -> String {
    ident.to_string().to_case(Case::Snake)
}
