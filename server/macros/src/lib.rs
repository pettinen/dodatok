use convert_case::{Case, Casing};
use proc_macro::TokenStream as CompilerTokenStream;
use proc_macro2::{Ident, Span, TokenStream as ProcMacro2TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseBuffer},
    parse_macro_input,
    token::Eq,
    Attribute, Expr, Fields, ItemEnum, LitStr, Variant,
};

#[proc_macro_attribute]
pub fn alert_enum(_meta: CompilerTokenStream, input: CompilerTokenStream) -> CompilerTokenStream {
    parse_macro_input!(input as AlertEnum)
        .to_token_stream()
        .into()
}

struct AlertEnum {
    enum_: ItemEnum,
    variants: Vec<AlertEnumVariant>,
}

impl Parse for AlertEnum {
    fn parse(input: &ParseBuffer) -> syn::Result<Self> {
        let enum_ = input.parse::<ItemEnum>()?;
        let variants = enum_
            .variants
            .clone()
            .into_iter()
            .map(|variant| AlertEnumVariant { variant })
            .collect();

        Ok(Self { enum_, variants })
    }
}

impl ToTokens for AlertEnum {
    fn to_tokens(&self, tokens: &mut ProcMacro2TokenStream) {
        let attrs = &self.enum_.attrs;
        let vis = &self.enum_.vis;
        let ident = &self.enum_.ident;
        let generics = &self.enum_.generics;
        let variants = &self.variants;
        let id_details_match_lines: Vec<_> = self.variants.iter()
            .map(|variant| variant.id_details_match_line()).collect();
        let length_details_match_lines: Vec<_> = self.variants.iter()
            .map(|variant| variant.length_details_match_line()).collect();
        let ident_string = ident.to_string();
        let source = match ident_string.strip_suffix("Error") {
            Some(source) => source.to_lowercase(),
            None => match ident_string.strip_suffix("Warning") {
                Some(source) => source.to_lowercase(),
                None => panic!("alert_enum name must end with 'Error' or 'Warning'"),
            },
        };

        tokens.extend(quote! {
            #(#attrs)*
            #[derive(Debug, thiserror::Error)]
            #vis enum #ident #generics {
                #(#variants),*
            }

            impl Error for #ident {
                fn to_tuple(&self) -> (String, String, Option<String>) {
                    let (id, details) = match self {
                        #(#id_details_match_lines),*
                    };
                    (#source.to_owned(), id, details)
                }
            }

            impl Serialize for #ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: Serializer
                {
                    let (length, details): (usize, Option<&String>) = match self {
                        #(#length_details_match_lines),*
                    };
                    let mut map = serializer.serialize_map(Some(length))?;
                    map.serialize_entry("source", #source)?;
                    map.serialize_entry("id", &self.to_string())?;
                    if let Some(details) = details {
                        map.serialize_entry("details", &details)?;
                    }
                    map.end()
                }
            }
        });
    }
}

struct AlertEnumVariant {
    variant: Variant,
}

impl AlertEnumVariant {
    fn id_details_match_line(&self) -> ProcMacro2TokenStream {
        let ident = &self.variant.ident;
        let (fields, details) = if self.variant.fields.is_empty() {
            (None, quote! { None })
        } else {
            (Some(quote! { (details) }), quote! { Some(details.to_owned()) })
        };
        let repr = ident.to_string().to_case(Case::Kebab);
        quote! {
            Self::#ident #fields => (#repr.to_owned(), #details)
        }
    }

    fn length_details_match_line(&self) -> ProcMacro2TokenStream {
        let ident = &self.variant.ident;
        if self.variant.fields.is_empty() {
            quote! { Self::#ident => (2, None) }
        } else {
            quote! { Self::#ident(details) => (3, Some(&details)) }
        }
    }
}

impl ToTokens for AlertEnumVariant {
    fn to_tokens(&self, tokens: &mut ProcMacro2TokenStream) {
        let attrs = &self.variant.attrs;
        let ident = &self.variant.ident;
        let kebab_case_name = ident.to_string().to_case(Case::Kebab);
        let fields = &self.variant.fields;

        tokens.extend(quote! {
            #(#attrs)*
            #[error(#kebab_case_name)]
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

#[proc_macro_attribute]
pub fn sql_enum(_meta: CompilerTokenStream, input: CompilerTokenStream) -> CompilerTokenStream {
    parse_macro_input!(input as SqlEnum)
        .to_token_stream()
        .into()
}

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
            .map(SqlEnumVariant::new)
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
    fn to_tokens(&self, tokens: &mut ProcMacro2TokenStream) {
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
