use crate::gtk_prelude::*;
use crate::utils::pending;
use std::path::{Path, PathBuf};

pub async fn select_dir(parent: &gtk::Window, pwd: &Path) -> Option<PathBuf> {
    let dlg = gtk::FileChooserDialog::builder()
        .transient_for(parent)
        .action(gtk::FileChooserAction::SelectFolder)
        .select_multiple(false)
        .resizable(true)
        .decorated(true)
        .build();

    dlg.add_button("_Cancel", gtk::ResponseType::Cancel);
    dlg.add_button("_Open", gtk::ResponseType::Accept);

    let f = gio::File::for_path(pwd);
    if let Err(err) = dlg.set_file(&f) {
        eprintln!("Cannot set default directory: {}", err);
    }

    let result = match dlg.run_future().await {
        gtk::ResponseType::Accept => dlg.file().and_then(|f| f.path()),
        _ => None,
    };

    dlg.close();
    pending().await;

    result
}

pub async fn save_as(parent: &gtk::Window, pwd: &Path) -> Option<PathBuf> {
    let dlg = gtk::FileChooserDialog::builder()
        .transient_for(parent)
        .action(gtk::FileChooserAction::Save)
        .select_multiple(false)
        .resizable(true)
        .decorated(true)
        .build();

    dlg.add_button("_Cancel", gtk::ResponseType::Cancel);
    dlg.add_button("_Save", gtk::ResponseType::Accept);

    let f = gio::File::for_path(pwd);
    if let Err(err) = dlg.set_current_folder(Some(&f)) {
        eprintln!("Cannot set default directory: {}", err);
    }

    let result = match dlg.run_future().await {
        gtk::ResponseType::Accept => dlg.file().and_then(|f| f.path()),
        _ => None,
    };

    dlg.close();
    pending().await;

    result
}
