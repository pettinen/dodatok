use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseBuffer},
    token::Eq,
    Attribute, Expr, Fields, ItemEnum, LitStr, Variant,
};

pub struct SqlEnum {
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
            .map(SqlEnumVariant::new)
            .collect();

        Ok(Self { enum_, variants })
    }
}

impl ToTokens for SqlEnum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attrs = &self.enum_.attrs;
        let vis = &self.enum_.vis;
        let ident = &self.enum_.ident;
        let generics = &self.enum_.generics;
        let snake_case_name = &self.enum_.ident.to_string().to_case(Case::Snake);
        let variants = &self.variants;
        let variant_names = variants.into_iter().map(|variant| &variant.rename);
        tokens.extend(quote! {
            #[derive(Debug, FromSql, ToSql, Serialize)]
            #[postgres(name = #snake_case_name)]
            #[serde(rename_all = "snake_case")]
            #(#attrs)*
            #vis enum #ident #generics {
                #(#variants),*
            }

            impl #ident {
                pub fn variants() -> Vec<String> {
                    vec![#(#variant_names),*].into_iter().map(|name| name.to_owned()).collect()
                }
            }
        })
    }
}

#[derive(Debug)]
struct SqlEnumVariant {
    attrs: Vec<Attribute>,
    discriminant: Option<(Eq, Expr)>,
    fields: Fields,
    ident: Ident,
    rename: String,
}

impl SqlEnumVariant {
    fn new(variant: Variant) -> Self {
        let name_ident = Ident::new("name", Span::call_site());
        let (name_attrs, other_attrs): (Vec<&Attribute>, Vec<&Attribute>) = variant
            .attrs
            .iter()
            .partition(|attr| attr.path.is_ident(&name_ident));
        if name_attrs.len() > 1 {
            panic!("multiple name(...) attributes specified for sql_enum variant");
        }
        let rename = match name_attrs.first() {
            Some(name_attr) => name_attr.parse_args::<LitStr>().unwrap().value(),
            None => variant.ident.to_string().to_case(Case::Snake),
        };
        Self {
            attrs: other_attrs.into_iter().cloned().collect(),
            discriminant: variant.discriminant,
            fields: variant.fields,
            ident: variant.ident,
            rename,
        }
    }
}

impl ToTokens for SqlEnumVariant {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attrs = &self.attrs;
        let fields = &self.fields;
        let ident = &self.ident;
        let rename = &self.rename;

        tokens.extend(quote! {
            #[postgres(name = #rename)]
            #[serde(rename = #rename)]
            #(#attrs)*
            #ident #fields
        });
        if let Some(discriminant) = &self.discriminant {
            let expr = &discriminant.1;
            tokens.extend(quote! {
                = #expr
            });
        }
    }
}
