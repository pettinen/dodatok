use convert_case::{Case, Casing};
use proc_macro::TokenStream as CompilerTokenStream;
use proc_macro2::{Ident, Span, TokenStream as ProcMacro2TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseBuffer},
    parse_macro_input, Attribute, ItemEnum, LitStr, Variant,
};

#[proc_macro_attribute]
pub fn error_enum(_meta: CompilerTokenStream, input: CompilerTokenStream) -> CompilerTokenStream {
    parse_macro_input!(input as ErrorEnum)
        .to_token_stream()
        .into()
}

struct ErrorEnum {
    enum_: ItemEnum,
    variants: Vec<ErrorEnumVariant>,
}

impl Parse for ErrorEnum {
    fn parse(input: &ParseBuffer) -> syn::Result<Self> {
        let enum_ = input.parse::<ItemEnum>()?;
        let variants = enum_
            .variants
            .clone()
            .into_iter()
            .map(|variant| ErrorEnumVariant { variant })
            .collect();

        Ok(Self { enum_, variants })
    }
}

impl ToTokens for ErrorEnum {
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
        let source = match ident.to_string().strip_suffix("Error") {
            Some(source) => source.to_lowercase(),
            None => panic!("error_enum name must end with 'Error'"),
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

struct ErrorEnumVariant {
    variant: Variant,
}

impl ErrorEnumVariant {
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

impl ToTokens for ErrorEnumVariant {
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
        let snake_case_name = &self.enum_.ident.to_string().to_case(Case::Snake);
        let variants = &self.variants;
        tokens.extend(quote! {
            #[derive(FromSql, Serialize)]
            #[postgres(name = #snake_case_name)]
            #[serde(rename_all = "snake_case")]
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
        let renames = if let Some(name_attr) = name_attrs.first() {
            let name = name_attr.parse_args::<LitStr>().unwrap().value();
            quote! {
                #[postgres(name = #name)]
                #[serde(rename = #name)]
            }
        } else {
            let snake_case_name = ident.to_string().to_case(Case::Snake);
            quote! {
                #[postgres(name = #snake_case_name)]
            }
        };

        tokens.extend(quote! {
            #renames
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
