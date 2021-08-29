use gtk::prelude::*;

pub fn go_button(label: &str) -> gtk::Button {
    let button = gtk::Button::builder().label(label).build();

    let style_provider = gtk::CssProvider::new();
    style_provider
        .load_from_data(b"button { padding-left: 48px; padding-right: 48px; }")
        .unwrap();

    let context = button.style_context();
    context.add_class(&gtk::STYLE_CLASS_SUGGESTED_ACTION);
    context.add_provider(&style_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

    button
}
