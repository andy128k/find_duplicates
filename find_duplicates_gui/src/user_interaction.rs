use gtk::prelude::*;
use std::time::Duration;

fn dialog(parent: &gtk::Window, title: &str) -> gtk::Dialog {
    gtk::Dialog::builder()
        .title(title)
        .transient_for(parent)
        .type_(gtk::WindowType::Toplevel)
        .type_hint(gdk::WindowTypeHint::Dialog)
        .modal(true)
        .window_position(gtk::WindowPosition::CenterOnParent)
        .resizable(false)
        .destroy_with_parent(false)
        .decorated(true)
        .gravity(gdk::Gravity::Center)
        .focus_on_map(true)
        .urgency_hint(false)
        .use_header_bar(1)
        .build()
}

pub fn prompt(parent: &gtk::Window, title: &str, message: &str, value: &str) -> Option<String> {
    let dlg = dialog(parent, title);

    dlg.add_button("Cancel", gtk::ResponseType::Cancel);
    dlg.add_button("Ok", gtk::ResponseType::Ok);
    dlg.set_default_response(gtk::ResponseType::Ok);

    let container = gtk::Box::builder()
        .homogeneous(false)
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin(20)
        .build();
    dlg.content_area().add(&container);

    let label = gtk::Label::builder()
        .label(message)
        .xalign(0.0_f32)
        .yalign(0.5_f32)
        .build();
    container.pack_start(&label, false, true, 0);

    let entry = gtk::Entry::builder()
        .text(value)
        .activates_default(true)
        .build();
    container.pack_start(&entry, false, true, 0);

    dlg.show_all();
    let result = match dlg.run() {
        gtk::ResponseType::Ok => Some(entry.text().to_string()),
        _ => None,
    };
    dlg.close();
    result
}

pub fn confirm_delete(parent: &gtk::Window, message: &str) -> (bool, bool) {
    let dlg = dialog(parent, "Delete");
    let yes = dlg.add_button("Delete", gtk::ResponseType::Ok);
    yes.style_context()
        .add_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);
    dlg.add_button("Cancel", gtk::ResponseType::Cancel);

    let container = gtk::Box::builder()
        .homogeneous(false)
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin(20)
        .build();
    dlg.content_area().add(&container);

    let label = gtk::Label::builder()
        .label(message)
        .xalign(0.0_f32)
        .yalign(0.5_f32)
        .build();
    container.pack_start(&label, false, true, 0);

    let again = gtk::CheckButton::builder()
        .label("Ask me this in future?")
        .active(true)
        .build();
    container.pack_start(&again, false, true, 0);

    dlg.show_all();
    let result = match dlg.run() {
        gtk::ResponseType::Ok => (true, again.is_active()),
        _ => (false, again.is_active()),
    };
    dlg.close();
    result
}

pub fn confirm(parent: &gtk::Window, message: &str) -> bool {
    let dlg = gtk::MessageDialog::builder()
        .message_type(gtk::MessageType::Question)
        .transient_for(parent)
        .text(message)
        .buttons(gtk::ButtonsType::YesNo)
        .build();
    dlg.show_all();
    let result = dlg.run();
    dlg.close();
    result == gtk::ResponseType::Yes
}

pub fn notify(message_type: gtk::MessageType, parent: &gtk::Window, message: &str) {
    let dlg = gtk::MessageDialog::builder()
        .message_type(message_type)
        .transient_for(parent)
        .text(message)
        .buttons(gtk::ButtonsType::Ok)
        .build();
    dlg.show_all();
    dlg.run();
    dlg.close();
}

pub fn notify_info(parent: &gtk::Window, message: &str) {
    notify(gtk::MessageType::Info, parent, message)
}

pub fn notify_error(parent: &gtk::Window, message: &str) {
    notify(gtk::MessageType::Error, parent, message)
}

pub fn notify_detailed(parent: &gtk::Window, message: &str, details: &str) {
    let dlg = gtk::MessageDialog::builder()
        .message_type(gtk::MessageType::Info)
        .transient_for(parent)
        .text(message)
        .buttons(gtk::ButtonsType::Ok)
        .width_request(600)
        .height_request(400)
        .resizable(true)
        .build();

    let scrolled_window = gtk::ScrolledWindow::builder()
        .can_focus(true)
        .margin_start(20)
        .margin_end(20)
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .shadow_type(gtk::ShadowType::EtchedIn)
        .window_placement(gtk::CornerType::TopLeft)
        .expand(true)
        .build();
    dlg.content_area()
        .pack_start(&scrolled_window, true, true, 0);

    let text_view = gtk::TextView::builder()
        .can_focus(true)
        .editable(false)
        .overwrite(false)
        .accepts_tab(true)
        .justification(gtk::Justification::Left)
        .wrap_mode(gtk::WrapMode::None)
        .cursor_visible(true)
        .left_margin(5)
        .right_margin(5)
        .top_margin(5)
        .bottom_margin(5)
        .build();
    text_view.buffer().unwrap().set_text(details);

    scrolled_window.add(&text_view);

    dlg.show_all();
    dlg.run();
    dlg.close();
}

pub fn progress(parent: &gtk::Window, title: &str) -> gtk::Dialog {
    let dlg = gtk::Dialog::builder()
        .title(title)
        .transient_for(parent)
        .type_(gtk::WindowType::Toplevel)
        .type_hint(gdk::WindowTypeHint::Dialog)
        .modal(true)
        .window_position(gtk::WindowPosition::CenterOnParent)
        .resizable(false)
        .destroy_with_parent(false)
        .decorated(true)
        .gravity(gdk::Gravity::Center)
        .focus_on_map(true)
        .urgency_hint(false)
        .use_header_bar(1)
        .deletable(false)
        .width_request(400)
        .build();

    let progress_bar = gtk::ProgressBar::builder().margin(30).build();

    dlg.content_area().add(&progress_bar);

    let weak_progress_bar = progress_bar.downgrade();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        if let Some(progress_bar) = weak_progress_bar.upgrade() {
            progress_bar.pulse();
            glib::Continue(true)
        } else {
            glib::Continue(false)
        }
    });

    dlg.show_all();

    dlg
}
