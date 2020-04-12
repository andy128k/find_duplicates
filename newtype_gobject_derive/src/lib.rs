extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, FieldsUnnamed};

#[proc_macro_derive(NewTypeGObject)]
pub fn newtype_gobject(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let newtype = input.ident;

    let wrapped_type = match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(FieldsUnnamed { ref unnamed, .. }),
            ..
        }) if unnamed.len() == 1 => &unnamed.first().unwrap().ty,
        _ => {
            panic!("#[derive(NewTypeGObject)] is only defined for newtype structs.");
        }
    };

    let expanded = quote! {
        impl std::convert::From<#wrapped_type> for #newtype {
            fn from(object: #wrapped_type) -> Self {
                Self(object)
            }
        }

        impl glib::clone::Downgrade for #newtype {
            type Weak = newtype_gobject::NewTypeWeakRef<#newtype, #wrapped_type>;

            fn downgrade(&self) -> Self::Weak {
                Self::Weak::from_inner(glib::clone::Downgrade::downgrade(&self.0))
            }
        }
    };

    TokenStream::from(expanded)
}
