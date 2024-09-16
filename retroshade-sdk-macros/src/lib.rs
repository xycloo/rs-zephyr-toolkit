use convert_case::Casing;
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    self, ext, parse_macro_input, DeriveInput, Expr, ExprLit, FieldsNamed, Ident, Lit, LitStr, Type,
};

#[proc_macro_derive(Retroshade)]
pub fn database_interact_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    let target = struct_name.to_string().to_case(convert_case::Case::Snake);

    // Actual trait implementation generation
    let expanded = quote! {
        impl #struct_name {
            pub fn emit(&self, env: &soroban_sdk::Env) {
                let target = soroban_sdk::Symbol::new(env, #target).as_val().get_payload() as i64;
                let event: soroban_sdk::Val = soroban_sdk::IntoVal::into_val(self, env);
                let event = event.get_payload() as i64;

                unsafe { retroshade_sdk::zephyr_emit(target, event) };
            }
        }
    };

    TokenStream::from(expanded)
}

#[test]
fn test() {}
