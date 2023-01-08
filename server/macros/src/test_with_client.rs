use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseBuffer},
    Ident, ItemFn,
};

pub struct TestWithClient {
    func: ItemFn,
}

impl Parse for TestWithClient {
    fn parse(input: &ParseBuffer) -> syn::Result<Self> {
        let func = input.parse::<ItemFn>()?;
        Ok(Self { func })
    }
}

impl ToTokens for TestWithClient {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let func = &self.func;
        let ident = &func.sig.ident;
        let name = ident.to_string();
        let context_name = Ident::new(
            &format!("{}Context", name.to_case(Case::Pascal)),
            Span::call_site(),
        );

        assert!(func.sig.inputs.len() == 0);
        assert!(func.sig.variadic.is_none());
        let attrs = &func.attrs;
        let vis = &func.vis;
        let constness = &func.sig.constness;
        let asyncness = &func.sig.asyncness;
        let unsafety = &func.sig.unsafety;
        let abi = &func.sig.abi;
        let generics = &func.sig.generics;
        let output = &func.sig.output;
        let block = &func.block;

        tokens.extend(quote! {
            struct #context_name {
                config: Config,
                db: ClientWrapper,
                endpoint: Box<dyn Endpoint<Output = Response>>,
            }

            #[async_trait]
            impl AsyncTestContext for #context_name {
                async fn setup() -> Self {
                    let (endpoint, db, config) = setup::init(#name).await;
                    Self {
                        config,
                        db,
                        endpoint: Box::new(endpoint),
                    }
                }

                async fn teardown(self) {
                    let mut db_config = self.config.db.clone();
                    db_config.dbname = Some("postgres".to_owned());
                    let pool = db_config.create_pool(None, NoTls).unwrap();
                    let db = pool.get().await.unwrap();
                    if let Err(err) = db.execute(&format!("DROP DATABASE {} (FORCE)", #name), &[]).await {
                       println!("{}", err);
                    }
                }
            }

            #[test_context(#context_name)]
            #[tokio::test]
            #(#attrs)*
            #vis #constness #asyncness #unsafety #abi fn #ident #generics (
                ctx: &#context_name
            ) #output
            #block
        })
    }
}
