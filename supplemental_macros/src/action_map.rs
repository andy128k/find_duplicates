use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{
    parse, spanned::Spanned, Attribute, Error, FnArg, Ident, ImplItem, ImplItemMethod, ItemImpl,
    Meta, MetaList, NestedMeta, PatType, Signature, Type,
};

#[derive(Debug, Default, FromMeta)]
#[darling(default)]
struct ActionAttributes {
    name: Option<String>,
}

#[derive(Debug)]
struct ActionInfo {
    attrs: ActionAttributes,
    sig: Signature,
}

fn generate_action(span: Span, action_name: &str, method: &Ident) -> TokenStream2 {
    quote_spanned! { span =>
        {
            let action = gio::SimpleAction::new(#action_name, None);
            action.connect_activate(
                glib::clone!(@weak self as this => move |_action, _parameter| {
                    this.#method();
                }),
            );
            action
        }
    }
}

fn generate_action_with_parameter(
    span: Span,
    action_name: &str,
    method: &Ident,
    parameter_type: &Type,
) -> TokenStream2 {
    quote_spanned! { span =>
        {
            let parameter_type = <#parameter_type as glib::variant::StaticVariantType>::static_variant_type();
            let action = gio::SimpleAction::new(#action_name, Some(&*parameter_type));
            action.connect_activate(
                glib::clone!(@weak self as this => move |_action, parameter| {
                    let parameter = match parameter {
                        Some(parameter) => parameter,
                        None => {
                            glib::g_critical!("actions", "Parameter of type {} is expected but none was passed to action {}.", parameter_type, #action_name);
                            return;
                        }
                    };
                    let parameter = match glib::variant::FromVariant::from_variant(parameter) {
                        Some(parameter) => parameter,
                        None => {
                            glib::g_critical!("actions", "Parameter of unexpected type {} is passed to action {} (Type \"{}\" is expected).", parameter.type_(), #action_name, parameter_type);
                            return;
                        }
                    };
                    this.#method(parameter);
                }),
            );
            action
        }
    }
}

fn generate_action_for_method(info: ActionInfo) -> Result<TokenStream2, Error> {
    let is_assoc = info
        .sig
        .inputs
        .first()
        .map_or(false, |arg| matches!(arg, FnArg::Receiver(..)));
    if !is_assoc {
        return Err(Error::new(
            info.sig.span(),
            "Unsupported signature of method. Only associated methods are supported.",
        ));
    }

    let method = &info.sig.ident;
    let action_name = info
        .attrs
        .name
        .as_ref()
        .map_or_else(|| method.to_string(), |name| name.clone());

    match info.sig.inputs.len() {
        1 => Ok(generate_action(info.sig.span(), &action_name, method)),
        2 => {
            let parameter_type = match &info.sig.inputs[1] {
                FnArg::Typed(PatType { ty, .. }) => ty,
                _ => return Err(Error::new(info.sig.span(), "Cannot extract type of a parameter")),
            };
            Ok(generate_action_with_parameter(info.sig.span(), &action_name, method, parameter_type))
        },
        n => Err(Error::new(
            info.sig.span(),
            format!("Unsupported signature of method. It has {} parameters but only 0 or 1 are supported.", n)
        )),
    }
}

fn combine_errors(error_acc: &mut Option<Error>, error: Error) {
    match error_acc {
        Some(ref mut error_acc) => {
            error_acc.combine(error);
        }
        None => {
            error_acc.replace(error);
        }
    }
}

fn attributes_to_metas(attributes: Vec<Attribute>) -> Result<Vec<NestedMeta>, Error> {
    let mut metas = Vec::new();
    let mut error = None;
    for attr in attributes {
        let meta = attr.parse_meta()?;
        match meta {
            Meta::List(MetaList { nested, .. }) => metas.extend(nested),
            _ => combine_errors(&mut error, Error::new(attr.span(), "Unexpected attribute")),
        }
    }
    if let Some(error) = error {
        Err(error)
    } else {
        Ok(metas)
    }
}

fn generate_register_method(actions: &[TokenStream2]) -> ImplItemMethod {
    let register_method = quote! {
        fn register_actions<AM: glib::object::IsA<gio::ActionMap>>(&self, map: &AM) {
            #(
                map.add_action(& #actions );
            )*
        }
    };
    parse(register_method.into()).unwrap()
}

pub fn actions(mut input: ItemImpl) -> Result<TokenStream, TokenStream> {
    let mut action_infos = Vec::new();
    for item in input.items.iter_mut() {
        if let ImplItem::Method(method) = item {
            let attributes =
                extract_from_vec(&mut method.attrs, |attr| attr.path.is_ident("action"));
            let metas = attributes_to_metas(attributes).map_err(|err| err.to_compile_error())?;
            action_infos.push(ActionInfo {
                attrs: ActionAttributes::from_list(&metas)
                    .map_err(|err| TokenStream::from(err.write_errors()))?,
                sig: method.sig.clone(),
            });
        }
    }

    let action_definitions: Vec<TokenStream2> = action_infos
        .into_iter()
        .map(generate_action_for_method)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_compile_error())?;

    let register_method = generate_register_method(&action_definitions);
    input.items.push(ImplItem::Method(register_method));

    Ok(quote!(#input).into())
}

// TODO: Replace this by Vec::drain_filter as soon as it is stabilized.
fn extract_from_vec<T>(vec: &mut Vec<T>, predicate: impl Fn(&T) -> bool) -> Vec<T> {
    let mut i = 0;
    let mut result: Vec<T> = Vec::new();
    while i != vec.len() {
        if (predicate)(&vec[i]) {
            let item = vec.remove(i);
            result.push(item);
        } else {
            i += 1;
        }
    }
    result
}
