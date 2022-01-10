use crate::gtk_prelude::*;
use crate::utils::pending;
use std::path::{Path, PathBuf};

pub async fn select_dir(parent: &gtk::Window, pwd: &Path) -> Option<PathBuf> {
    let dlg = gtk::FileChooserDialog::builder()
        .transient_for(parent)
        .local_only(true)
        .border_width(5)
        .action(gtk::FileChooserAction::SelectFolder)
        .select_multiple(false)
        .type_(gtk::WindowType::Toplevel)
        .type_hint(gdk::WindowTypeHint::Dialog)
        .resizable(true)
        .decorated(true)
        .focus_on_map(true)
        .build();

    dlg.add_button("_Cancel", gtk::ResponseType::Cancel);
    dlg.add_button("_Open", gtk::ResponseType::Accept);

    dlg.select_filename(pwd);

    let result = match dlg.run_future().await {
        gtk::ResponseType::Accept => dlg.filename(),
        _ => None,
    };

    dlg.close();
    pending().await;

    result
}

pub async fn save_as(parent: &gtk::Window, pwd: &Path) -> Option<PathBuf> {
    let dlg = gtk::FileChooserDialog::builder()
        .transient_for(parent)
        .local_only(true)
        .border_width(5)
        .action(gtk::FileChooserAction::Save)
        .select_multiple(false)
        .type_(gtk::WindowType::Toplevel)
        .type_hint(gdk::WindowTypeHint::Dialog)
        .resizable(true)
        .decorated(true)
        .focus_on_map(true)
        .build();

    dlg.add_button("_Cancel", gtk::ResponseType::Cancel);
    dlg.add_button("_Save", gtk::ResponseType::Accept);

    dlg.set_current_folder(pwd);

    let result = match dlg.run_future().await {
        gtk::ResponseType::Accept => dlg.filename(),
        _ => None,
    };

    dlg.close();
    pending().await;

    result
}
