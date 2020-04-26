use gtk::prelude::*;

fn dialog(parent: &gtk::Window, title: &str) -> gtk::Dialog {
    let dialog = gtk::DialogBuilder::new()
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
        .build();

    dialog
}

pub fn prompt(parent: &gtk::Window, title: &str, message: &str, value: &str) -> Option<String> {
    let dlg = dialog(parent, title);

    dlg.add_button("Cancel", gtk::ResponseType::Cancel);
    dlg.add_button("Ok", gtk::ResponseType::Ok);
    dlg.set_default_response(gtk::ResponseType::Ok);

    let container = gtk::BoxBuilder::new()
        .homogeneous(false)
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin(20)
        .build();
    dlg.get_content_area().add(&container);

    let label = gtk::LabelBuilder::new()
        .label(message)
        .xalign(0.0_f32)
        .yalign(0.5_f32)
        .build();
    container.pack_start(&label, false, true, 0);

    let entry = gtk::EntryBuilder::new()
        .text(value)
        .activates_default(true)
        .build();
    container.pack_start(&entry, false, true, 0);

    dlg.show_all();
    let result = match dlg.run() {
        gtk::ResponseType::Ok => entry.get_text().map(|s| s.to_string()),
        _ => None,
    };
    dlg.destroy();
    result
}

pub fn confirm_delete(parent: &gtk::Window, message: &str) -> (bool, bool) {
    let dlg = dialog(parent, "Delete");
    let yes = dlg.add_button("Delete", gtk::ResponseType::Ok);
    yes.get_style_context()
        .add_class(&gtk::STYLE_CLASS_DESTRUCTIVE_ACTION);
    dlg.add_button("Cancel", gtk::ResponseType::Cancel);

    let container = gtk::BoxBuilder::new()
        .homogeneous(false)
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin(20)
        .build();
    dlg.get_content_area().add(&container);

    let label = gtk::LabelBuilder::new()
        .label(message)
        .xalign(0.0_f32)
        .yalign(0.5_f32)
        .build();
    container.pack_start(&label, false, true, 0);

    let again = gtk::CheckButtonBuilder::new()
        .label("Ask me this in future?")
        .active(true)
        .build();
    container.pack_start(&again, false, true, 0);

    dlg.show_all();
    let result = match dlg.run() {
        gtk::ResponseType::Ok => (true, again.get_active()),
        _ => (false, again.get_active()),
    };
    dlg.destroy();
    result
}

pub fn confirm(parent: &gtk::Window, message: &str) -> bool {
    let dlg = gtk::MessageDialogBuilder::new()
        .message_type(gtk::MessageType::Question)
        .transient_for(parent)
        .text(message)
        .buttons(gtk::ButtonsType::YesNo)
        .build();
    dlg.show_all();
    let result = dlg.run();
    dlg.destroy();
    result == gtk::ResponseType::Yes
}

pub fn notify(message_type: gtk::MessageType, parent: &gtk::Window, message: &str) {
    let dlg = gtk::MessageDialogBuilder::new()
        .message_type(message_type)
        .transient_for(parent)
        .text(message)
        .buttons(gtk::ButtonsType::Ok)
        .build();
    dlg.show_all();
    dlg.run();
    dlg.destroy();
}

pub fn notify_info(parent: &gtk::Window, message: &str) {
    notify(gtk::MessageType::Info, parent, message)
}

pub fn notify_error(parent: &gtk::Window, message: &str) {
    notify(gtk::MessageType::Error, parent, message)
}

pub fn progress(parent: &gtk::Window, title: &str) -> gtk::Dialog {
    let dlg = gtk::DialogBuilder::new()
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

    let progress_bar = gtk::ProgressBarBuilder::new().margin(30).build();

    dlg.get_content_area().add(&progress_bar);

    let weak_progress_bar = progress_bar.downgrade();
    gtk::timeout_add(100, move || {
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
