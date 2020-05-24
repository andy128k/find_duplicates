extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, format_ident};
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

    let weak_ref = format_ident!("{}WeakRef", newtype);

    let expanded = quote! {
        pub struct #weak_ref(glib::object::WeakRef<#wrapped_type>);

        impl glib::clone::Downgrade for #newtype {
            type Weak = #weak_ref;

            fn downgrade(&self) -> Self::Weak {
                #weak_ref(glib::clone::Downgrade::downgrade(&self.0))
            }
        }

        impl glib::clone::Upgrade for #weak_ref {
            type Strong = #newtype;

            fn upgrade(&self) -> Option<Self::Strong> {
                glib::clone::Upgrade::upgrade(&self.0).map(|upgraded_inner| #newtype(upgraded_inner))
            }
        }
    };

    TokenStream::from(expanded)
}
