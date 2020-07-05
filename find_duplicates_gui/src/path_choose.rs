use gtk::prelude::*;
use std::path::{Path, PathBuf};

pub fn select_dir(parent: &gtk::Window, pwd: &Path) -> Option<PathBuf> {
    let dlg = gtk::FileChooserDialogBuilder::new()
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

    let result = match dlg.run() {
        gtk::ResponseType::Accept => dlg.get_filename(),
        _ => None,
    };

    dlg.close();

    result
}

pub fn save_as(parent: &gtk::Window, pwd: &Path) -> Option<PathBuf> {
    let dlg = gtk::FileChooserDialogBuilder::new()
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

    let result = match dlg.run() {
        gtk::ResponseType::Accept => dlg.get_filename(),
        _ => None,
    };

    dlg.close();

    result
}
