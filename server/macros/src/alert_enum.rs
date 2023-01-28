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
    is_response_error: Option<bool>,
}

impl AlertEnum {
    pub fn set_response_error(&mut self, is_response_error: bool) {
        self.is_response_error = Some(is_response_error);
        for variant in &mut self.variants {
            variant.set_response_error(is_response_error);
        }
    }
}

impl Parse for AlertEnum {
    fn parse(input: &ParseBuffer) -> syn::Result<Self> {
        let enum_ = input.parse::<ItemEnum>()?;
        let variants = enum_
            .variants
            .clone()
            .into_iter()
            .map(AlertEnumVariant::new)
            .collect();

        Ok(Self {
            enum_,
            variants,
            is_response_error: None,
        })
    }
}

impl ToTokens for AlertEnum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attrs = &self.enum_.attrs;
        let vis = &self.enum_.vis;
        let ident = &self.enum_.ident;
        let generics = &self.enum_.generics;
        let variants = &self.variants;
        let id_data_match_lines: Vec<_> = self
            .variants
            .iter()
            .map(|variant| variant.id_data_match_line())
            .collect();
        let details_match_lines: Vec<_> = self
            .variants
            .iter()
            .map(|variant| variant.details_match_line())
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

            impl #ident {
                fn data(&self) -> std::option::Option<crate::error::ErrorData> {
                    let (_, data) = match self {
                        #(#id_data_match_lines),*
                    };
                    data.to_owned()
                }

                fn to_tuple(&self) -> (
                    std::string::String,
                    std::string::String,
                    std::option::Option<std::string::String>
                ) {
                    let (id, data) = match self {
                        #(#id_data_match_lines),*
                    };
                    (#source.to_owned(), id, data.clone().map(|data| data.details).flatten())
                }
            }
        });

        if self.is_response_error.unwrap() {
            tokens.extend(quote! {
                impl #ident {
                    fn as_response(&self) -> poem::Response {
                        let (src, id, details) = self.to_tuple();
                        let data = self.data();
                        let mut body = crate::error::single_error(&src, &id, details);
                        let mut res = poem::Response::builder()
                            .status(self.status())
                            .content_type("application/json");

                        if let Some(data) = data {
                            if let Some((csrf_response_field, csrf_token)) = data.csrf_token {
                                match body.as_object_mut() {
                                    Some(body) => {
                                        body.insert(
                                            csrf_response_field,
                                            serde_json::Value::String(csrf_token),
                                        );
                                    }
                                    None => {
                                        return InternalError::new("single_error returned non-object")
                                            .as_response();
                                    }
                                }
                            }

                            for cookie in data.cookies {
                                res = res.header(
                                    poem::http::header::SET_COOKIE,
                                    cookie.to_string()
                                );
                            }
                        }

                        res.body(poem::Body::from_json(body).unwrap())
                    }
                }
            });
        }

        tokens.extend(quote! {
            impl serde::Serialize for #ident {
                fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
                where S: serde::Serializer
                {
                    let details = match self {
                        #(#details_match_lines),*
                    };
                    let mut map = if details.is_some() {
                        serializer.serialize_map(std::option::Option::Some(3))?
                    } else {
                        serializer.serialize_map(std::option::Option::Some(2))?
                    };
                    map.serialize_entry("source", #source)?;
                    map.serialize_entry("id", &self.to_string())?;
                    if let std::option::Option::Some(details) = details {
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
    is_response_error: Option<bool>,
}

impl AlertEnumVariant {
    fn new(variant: Variant) -> Self {
        Self {
            variant,
            is_response_error: None,
        }
    }

    fn set_response_error(&mut self, is_response_error: bool) {
        self.is_response_error = Some(is_response_error);
    }

    fn id_data_match_line(&self) -> TokenStream {
        let ident = &self.variant.ident;
        let repr = ident.to_string().to_case(Case::Kebab);
        quote! {
            Self::#ident(data) => (#repr.to_owned(), data)
        }
    }

    fn details_match_line(&self) -> TokenStream {
        let ident = &self.variant.ident;
        quote! {
            Self::#ident(
                std::option::Option::Some(
                    crate::error::ErrorData { details, .. }
                )
            ) => details,
            Self::#ident(std::option::Option::None) =>
                &std::option::Option::None
        }
    }
}

impl ToTokens for AlertEnumVariant {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attrs = &self.variant.attrs;
        let ident = &self.variant.ident;
        let kebab_case_name = ident.to_string().to_case(Case::Kebab);
        if !self.variant.fields.is_empty() {
            panic!("unexpected alert_enum variant fields");
        }

        tokens.extend(quote! {
            #(#attrs)*
            #[error(#kebab_case_name)]
            #ident(std::option::Option<crate::error::ErrorData>)
        });
        if let Some(discriminant) = &self.variant.discriminant {
            let expr = &discriminant.1;
            tokens.extend(quote! {
                = #expr
            });
        }
    }
}
