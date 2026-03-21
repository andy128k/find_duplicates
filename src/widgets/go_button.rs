use gtk::prelude::*;

pub fn go_button(label: &str, action: &str) -> gtk::Button {
    let button = gtk::Button::builder()
        .label(label)
        .action_name(action)
        .build();

    button.add_css_class("go-button");
    button.add_css_class("suggested-action");
    button
}
