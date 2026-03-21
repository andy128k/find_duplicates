use gtk::{gio, prelude::*};
use std::path::{Path, PathBuf};

pub async fn select_dir(parent: &gtk::Window, pwd: &Path) -> Option<PathBuf> {
    let choice = gtk::FileDialog::builder()
        .modal(true)
        .initial_folder(&gio::File::for_path(pwd))
        .build()
        .select_folder_future(Some(parent))
        .await;
    match choice {
        Ok(file) => file.path(),
        Err(error) => {
            eprintln!("Failed to select a directory file: {error}");
            None
        }
    }
}

pub async fn save_as(parent: &gtk::Window, pwd: &Path) -> Option<PathBuf> {
    let choice = gtk::FileDialog::builder()
        .modal(true)
        .initial_folder(&gio::File::for_path(pwd))
        .build()
        .save_future(Some(parent))
        .await;
    match choice {
        Ok(file) => file.path(),
        Err(error) => {
            eprintln!("Failed to select a destination file: {error}");
            None
        }
    }
}
