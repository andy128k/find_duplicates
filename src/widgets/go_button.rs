use gtk::{gdk, prelude::*};
use std::sync::Once;

fn inject_css(display: &gdk::Display, style: &str) {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(style);
    gtk::style_context_add_provider_for_display(
        display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

const CSS: &str = r#"
.go-button {
    padding-left: 48px;
    padding-right: 48px;
}
"#;

pub fn go_button(label: &str, action: &str) -> gtk::Button {
    let button = gtk::Button::builder()
        .label(label)
        .action_name(action)
        .build();

    button.add_css_class("go-button");
    button.add_css_class("suggested-action");

    button.connect_realize(move |widget| {
        let display = widget.display();
        static ONCE: Once = Once::new();
        ONCE.call_once(|| inject_css(&display, CSS));
    });
    button
}
