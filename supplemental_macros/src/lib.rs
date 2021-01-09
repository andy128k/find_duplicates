extern crate proc_macro;

mod downgrade_enum;
mod downgrade_fields;
mod downgrade_struct;

use proc_macro::TokenStream;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(GlibDowngrade)]
pub fn newtype_gobject(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match input.data {
        Data::Struct(data) => {
            downgrade_struct::derive_downgrade_for_struct(input.ident, input.generics, data)
        }
        Data::Enum(data) => downgrade_enum::derive_downgrade_for_enum(input.ident, data),
        Data::Union(..) => {
            panic!("#[derive(GlibDowngrade)] is not available for unions.");
        }
    }
}
