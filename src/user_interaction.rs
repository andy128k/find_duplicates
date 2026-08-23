use crate::utils::{SenderExt, pending, scrolled};
use gtk::{glib, prelude::*};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

pub async fn prompt(
    parent: &gtk::Window,
    title: &str,
    message: &str,
    value: &str,
) -> Option<String> {
    let dlg = gtk::Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .resizable(false)
        .destroy_with_parent(false)
        .decorated(true)
        .width_request(300)
        .build();

    let container = gtk::Box::builder()
        .homogeneous(false)
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    dlg.set_child(Some(&container));

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

    let (sender, receiver) = async_channel::bounded::<bool>(1);
    dlg.connect_destroy(glib::clone!(
        #[strong]
        sender,
        move |_| sender.toss(false)
    ));

    let button_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .vexpand(true)
        .valign(gtk::Align::End)
        .build();
    container.append(&button_box);
    let button_group = gtk::SizeGroup::new(gtk::SizeGroupMode::Both);

    let cancel_button = gtk::Button::builder()
        .label("Cancel")
        .hexpand(true)
        .halign(gtk::Align::End)
        .build();
    button_box.append(&cancel_button);
    button_group.add_widget(&cancel_button);
    cancel_button.connect_clicked(glib::clone!(
        #[strong]
        sender,
        move |_| sender.toss(false)
    ));

    let ok_button = gtk::Button::builder()
        .label("OK")
        .receives_default(true)
        .build();
    button_box.append(&ok_button);
    button_group.add_widget(&ok_button);
    ok_button.connect_clicked(glib::clone!(
        #[strong]
        sender,
        move |_| sender.toss(true)
    ));

    dlg.present();
    let result = match receiver.recv().await {
        Ok(true) => Some(entry.text().to_string()),
        _ => None,
    };
    dlg.close();
    pending().await;
    result
}

pub async fn confirm_delete(parent: &gtk::Window, message: &str) -> (bool, bool) {
    let dlg = gtk::Window::builder()
        .title("Delete")
        .transient_for(parent)
        .modal(true)
        .resizable(false)
        .destroy_with_parent(false)
        .decorated(true)
        .width_request(300)
        .build();

    let container = gtk::Box::builder()
        .homogeneous(false)
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    dlg.set_child(Some(&container));

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

    let (sender, receiver) = async_channel::bounded::<bool>(1);
    dlg.connect_destroy(glib::clone!(
        #[strong]
        sender,
        move |_| sender.toss(false)
    ));

    let button_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .vexpand(true)
        .valign(gtk::Align::End)
        .build();
    container.append(&button_box);
    let button_group = gtk::SizeGroup::new(gtk::SizeGroupMode::Both);

    let cancel_button = gtk::Button::builder()
        .label("Cancel")
        .hexpand(true)
        .halign(gtk::Align::End)
        .build();
    button_box.append(&cancel_button);
    button_group.add_widget(&cancel_button);
    cancel_button.connect_clicked(glib::clone!(
        #[strong]
        sender,
        move |_| sender.toss(false)
    ));

    let ok_button = gtk::Button::builder()
        .label("Delete")
        .receives_default(true)
        .build();
    ok_button.add_css_class("destructive-action");
    button_box.append(&ok_button);
    button_group.add_widget(&ok_button);
    ok_button.connect_clicked(glib::clone!(
        #[strong]
        sender,
        move |_| sender.toss(true)
    ));

    dlg.present();
    let allow_delete = matches!(receiver.recv().await, Ok(true));
    let ask_again = again.is_active();
    dlg.close();
    pending().await;

    (allow_delete, ask_again)
}

pub async fn confirm(parent: &gtk::Window, message: &str) -> bool {
    let dlg = gtk::AlertDialog::builder()
        .modal(true)
        .message(message)
        .buttons(["No", "Yes"])
        .cancel_button(0)
        .default_button(1)
        .build();
    let result = dlg.choose_future(Some(parent)).await;
    result == Ok(1)
}

pub async fn notify(_message_type: gtk::MessageType, parent: &gtk::Window, message: &str) {
    let _ = gtk::AlertDialog::builder()
        .modal(true)
        .message(message)
        .buttons(["OK"])
        .default_button(0)
        .cancel_button(0)
        .build()
        .choose_future(Some(parent))
        .await;
}

pub async fn notify_info(parent: &gtk::Window, message: &str) {
    notify(gtk::MessageType::Info, parent, message).await;
}

pub async fn notify_error(parent: &gtk::Window, message: &str) {
    notify(gtk::MessageType::Error, parent, message).await;
}

pub async fn notify_detailed(parent: &gtk::Window, message: &str, details: &str) {
    let dlg = gtk::Window::builder()
        .title("Delete")
        .transient_for(parent)
        .modal(true)
        .resizable(true)
        .destroy_with_parent(false)
        .decorated(true)
        .width_request(600)
        .height_request(400)
        .build();

    let container = gtk::Box::builder()
        .homogeneous(false)
        .orientation(gtk::Orientation::Vertical)
        .spacing(6)
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    dlg.set_child(Some(&container));

    let label = gtk::Label::builder()
        .label(message)
        .xalign(0.0_f32)
        .yalign(0.5_f32)
        .build();
    container.append(&label);

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

    let scrolled_window = scrolled(&text_view, true);
    scrolled_window.set_parent(&container);

    let (sender, receiver) = async_channel::bounded::<()>(1);
    dlg.connect_destroy(glib::clone!(
        #[strong]
        sender,
        move |_| sender.toss(())
    ));

    let button_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .vexpand(true)
        .valign(gtk::Align::End)
        .build();
    container.append(&button_box);

    let ok_button = gtk::Button::builder()
        .label("OK")
        .receives_default(true)
        .build();
    button_box.append(&ok_button);
    ok_button.connect_clicked(glib::clone!(
        #[strong]
        sender,
        move |_| sender.toss(())
    ));

    dlg.present();
    let _ = receiver.recv().await;
    dlg.close();

    pending().await;
}

pub struct ProgressDialog {
    dlg: gtk::Window,
    running: Rc<Cell<bool>>,
}

impl ProgressDialog {
    pub fn new(parent: &gtk::Window, title: &str) -> Self {
        let dlg = gtk::Window::builder()
            .title(title)
            .transient_for(parent)
            .modal(true)
            .resizable(false)
            .destroy_with_parent(false)
            .decorated(true)
            .deletable(false)
            .width_request(400)
            .build();

        let progress_bar = gtk::ProgressBar::builder()
            .margin_start(30)
            .margin_end(30)
            .margin_top(30)
            .margin_bottom(30)
            .build();
        dlg.set_child(Some(&progress_bar));

        let running = Rc::new(Cell::new(true));
        dlg.connect_close_request(glib::clone!(
            #[weak]
            running,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_dlg| {
                if running.get() {
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
        ));

        glib::timeout_add_local(
            Duration::from_millis(100),
            glib::clone!(
                #[weak]
                progress_bar,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    progress_bar.pulse();
                    glib::ControlFlow::Continue
                }
            ),
        );

        dlg.present();

        Self { dlg, running }
    }

    pub async fn close(self) {
        self.running.set(false);
        self.dlg.close();
        pending().await;
    }
}
