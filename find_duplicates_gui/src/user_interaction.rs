use crate::gtk_prelude::*;
use crate::utils::pending;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

fn dialog(parent: &gtk::Window, title: &str) -> gtk::Dialog {
    gtk::Dialog::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .resizable(false)
        .destroy_with_parent(false)
        .decorated(true)
        .use_header_bar(1)
        .build()
}

pub async fn prompt(
    parent: &gtk::Window,
    title: &str,
    message: &str,
    value: &str,
) -> Option<String> {
    let dlg = dialog(parent, title);

    dlg.add_button("Cancel", gtk::ResponseType::Cancel);
    dlg.add_button("Ok", gtk::ResponseType::Ok);
    dlg.set_default_response(gtk::ResponseType::Ok);

    let container = gtk::Box::builder()
        .homogeneous(false)
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_start(20)
        .margin_end(20)
        .margin_top(20)
        .margin_bottom(20)
        .build();
    container.set_parent(&dlg.content_area());

    let label = gtk::Label::builder()
        .label(message)
        .xalign(0.0_f32)
        .yalign(0.5_f32)
        .build();
    container.append(&label);

    let entry = gtk::Entry::builder()
        .text(value)
        .activates_default(true)
        .build();
    container.append(&entry);

    dlg.show();
    let result = match dlg.run_future().await {
        gtk::ResponseType::Ok => Some(entry.text().to_string()),
        _ => None,
    };
    dlg.close();
    pending().await;
    result
}

pub async fn confirm_delete(parent: &gtk::Window, message: &str) -> (bool, bool) {
    let dlg = dialog(parent, "Delete");
    let yes = dlg.add_button("Delete", gtk::ResponseType::Ok);
    yes.style_context().add_class("destructive-action");
    dlg.add_button("Cancel", gtk::ResponseType::Cancel);

    let container = gtk::Box::builder()
        .homogeneous(false)
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_start(20)
        .margin_end(20)
        .margin_top(20)
        .margin_bottom(20)
        .build();
    container.set_parent(&dlg.content_area());

    let label = gtk::Label::builder()
        .label(message)
        .xalign(0.0_f32)
        .yalign(0.5_f32)
        .build();
    container.append(&label);

    let again = gtk::CheckButton::builder()
        .label("Ask me this in future?")
        .active(true)
        .build();
    container.append(&again);

    dlg.show();
    let response = dlg.run_future().await;
    let ask_again = again.is_active();
    dlg.close();
    pending().await;

    let allow_delete = response == gtk::ResponseType::Ok;
    (allow_delete, ask_again)
}

pub async fn confirm(parent: &gtk::Window, message: &str) -> bool {
    let dlg = gtk::MessageDialog::builder()
        .message_type(gtk::MessageType::Question)
        .transient_for(parent)
        .text(message)
        .buttons(gtk::ButtonsType::YesNo)
        .build();
    dlg.show();
    let result = dlg.run_future().await;
    dlg.close();
    pending().await;
    result == gtk::ResponseType::Yes
}

pub async fn notify(message_type: gtk::MessageType, parent: &gtk::Window, message: &str) {
    let dlg = gtk::MessageDialog::builder()
        .message_type(message_type)
        .transient_for(parent)
        .text(message)
        .buttons(gtk::ButtonsType::Ok)
        .build();
    dlg.show();
    dlg.run_future().await;
    dlg.close();
    pending().await;
}

pub async fn notify_info(parent: &gtk::Window, message: &str) {
    notify(gtk::MessageType::Info, parent, message).await;
}

pub async fn notify_error(parent: &gtk::Window, message: &str) {
    notify(gtk::MessageType::Error, parent, message).await;
}

pub async fn notify_detailed(parent: &gtk::Window, message: &str, details: &str) {
    let dlg = gtk::MessageDialog::builder()
        .message_type(gtk::MessageType::Info)
        .transient_for(parent)
        .text(message)
        .buttons(gtk::ButtonsType::Ok)
        .width_request(600)
        .height_request(400)
        .resizable(true)
        .build();

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
    text_view.buffer().set_text(details);

    let scrolled_window = gtk::ScrolledWindow::builder()
        .can_focus(true)
        .margin_start(20)
        .margin_end(20)
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .has_frame(true)
        .window_placement(gtk::CornerType::TopLeft)
        .hexpand(true)
        .vexpand(true)
        .child(&text_view)
        .build();
    scrolled_window.set_parent(&dlg.content_area());

    dlg.show();
    dlg.run_future().await;
    dlg.close();
    pending().await;
}

pub struct ProgressDialog {
    dlg: gtk::Dialog,
    running: Rc<Cell<bool>>,
}

impl ProgressDialog {
    pub fn new(parent: &gtk::Window, title: &str) -> Self {
        let dlg = gtk::Dialog::builder()
            .title(title)
            .transient_for(parent)
            .modal(true)
            .resizable(false)
            .destroy_with_parent(false)
            .decorated(true)
            .use_header_bar(1)
            .deletable(false)
            .width_request(400)
            .build();

        let progress_bar = gtk::ProgressBar::builder()
            .margin_start(30)
            .margin_end(30)
            .margin_top(30)
            .margin_bottom(30)
            .build();

        progress_bar.set_parent(&dlg.content_area());

        let running = Rc::new(Cell::new(true));
        dlg.connect_close_request(
            clone!(@weak running => @default-return glib::signal::Inhibit(false), move |_dlg| {
                glib::signal::Inhibit(running.get())
            }),
        );

        let weak_progress_bar = progress_bar.downgrade();
        glib::timeout_add_local(Duration::from_millis(100), move || {
            if let Some(progress_bar) = weak_progress_bar.upgrade() {
                progress_bar.pulse();
                glib::Continue(true)
            } else {
                glib::Continue(false)
            }
        });

        dlg.show();

        Self { dlg, running }
    }

    pub async fn close(self) {
        self.running.set(false);
        self.dlg.close();
        pending().await;
    }
}
