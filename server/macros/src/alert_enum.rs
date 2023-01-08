use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseBuffer},
    ItemEnum, Variant,
};

pub struct AlertEnum {
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
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attrs = &self.enum_.attrs;
        let vis = &self.enum_.vis;
        let ident = &self.enum_.ident;
        let generics = &self.enum_.generics;
        let variants = &self.variants;
        let id_details_match_lines: Vec<_> = self
            .variants
            .iter()
            .map(|variant| variant.id_details_match_line())
            .collect();
        let length_details_match_lines: Vec<_> = self
            .variants
            .iter()
            .map(|variant| variant.length_details_match_line())
            .collect();
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
    fn id_details_match_line(&self) -> TokenStream {
        let ident = &self.variant.ident;
        let (fields, details) = if self.variant.fields.is_empty() {
            (None, quote! { None })
        } else {
            (
                Some(quote! { (details) }),
                quote! { Some(details.to_owned()) },
            )
        };
        let repr = ident.to_string().to_case(Case::Kebab);
        quote! {
            Self::#ident #fields => (#repr.to_owned(), #details)
        }
    }

    fn length_details_match_line(&self) -> TokenStream {
        let ident = &self.variant.ident;
        if self.variant.fields.is_empty() {
            quote! { Self::#ident => (2, None) }
        } else {
            quote! { Self::#ident(details) => (3, Some(&details)) }
        }
    }
}

impl ToTokens for AlertEnumVariant {
    fn to_tokens(&self, tokens: &mut TokenStream) {
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
