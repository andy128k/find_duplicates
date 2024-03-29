use crate::exclusion::Exclusion;
use crate::gtk_prelude::*;
use crate::path_choose::select_dir;
use crate::string_list::StringList;
use crate::user_interaction::prompt;
use crate::utils::{horizontal_expander, scrolled};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::string::ToString;

#[derive(Clone, Serialize, Deserialize)]
struct Directory(PathBuf);

impl ToString for Directory {
    fn to_string(&self) -> String {
        self.0.display().to_string()
    }
}

fn form_label(label: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(label)
        .justify(gtk::Justification::Left)
        .wrap(false)
        .selectable(false)
        .hexpand(false)
        .xalign(0.0_f32)
        .yalign(0.5_f32)
        .build()
}

#[derive(Clone)]
pub struct Options {
    container: gtk::Grid,
    directories: StringList<Directory>,
    excluded: StringList<Exclusion>,
    recurse: gtk::CheckButton,
    min_size: gtk::Entry,
}

fn get_window(widget: &impl IsA<gtk::Widget>) -> Option<gtk::Window> {
    widget.root()?.downcast::<gtk::Window>().ok()
}

async fn pick_directory(window: &gtk::Window) -> Option<PathBuf> {
    let pwd = std::env::current_dir().ok()?;
    let path = select_dir(&window, &pwd).await?;
    Some(path)
}

async fn pick_pattern(window: &gtk::Window) -> Option<String> {
    let pattern = prompt(&window, "Add pattern", "pattern", "").await?;
    if pattern.is_empty() {
        None
    } else {
        Some(pattern)
    }
}

fn add_directory_button(string_list: &StringList<Directory>) -> gtk::Button {
    let button = gtk::Button::builder()
        .label("Add directory")
        .hexpand(false)
        .build();
    button.connect_clicked(clone!(@weak string_list => move |button| {
        if let Some(window) = get_window(button) {
            glib::MainContext::default().spawn_local(async move {
                if let Some(new_value) = pick_directory(&window).await {
                    string_list.append(Directory(new_value));
                }
            });
        }
    }));
    button
}

fn add_excluded_directory_button(string_list: &StringList<Exclusion>) -> gtk::Button {
    let button = gtk::Button::builder()
        .label("Add directory")
        .hexpand(false)
        .build();
    button.connect_clicked(clone!(@weak string_list => move |button| {
        if let Some(window) = get_window(button) {
            glib::MainContext::default().spawn_local(async move {
                if let Some(new_value) = pick_directory(&window).await {
                    string_list.append(Exclusion::Directory(new_value));
                }
            });
        }
    }));
    button
}

fn add_exclusion_pattern_button(string_list: &StringList<Exclusion>) -> gtk::Button {
    let button = gtk::Button::builder()
        .label("Add pattern")
        .hexpand(false)
        .build();
    button.connect_clicked(clone!(@weak string_list => move |button| {
        if let Some(window) = get_window(button) {
            glib::MainContext::default().spawn_local(async move {
                if let Some(new_value) = pick_pattern(&window).await {
                    string_list.append(Exclusion::Pattern(new_value));
                }
            });
        }
    }));
    button
}

fn remove_selection_button<T: ToString + 'static>(string_list: &StringList<T>) -> gtk::Button {
    let button = gtk::Button::builder()
        .label("Remove")
        .hexpand(false)
        .build();
    button.connect_clicked(clone!(@weak string_list => move |_|
        string_list.remove_selection()
    ));
    button
}

fn clear_button<T: ToString + 'static>(string_list: &StringList<T>) -> gtk::Button {
    let button = gtk::Button::builder().label("Clear").hexpand(false).build();
    button.connect_clicked(clone!(@weak string_list => move |_|
        string_list.clear()
    ));
    button
}

fn button_column(buttons: &[gtk::Button]) -> gtk::Widget {
    let container = gtk::Box::builder()
        .homogeneous(false)
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .build();

    for button in buttons {
        container.append(button);
    }

    container.upcast()
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

impl Options {
    pub fn new() -> Self {
        let container = gtk::Grid::builder()
            .column_homogeneous(false)
            .row_homogeneous(false)
            .column_spacing(8)
            .row_spacing(8)
            .build();

        let directories_label = form_label("Directories to search");
        container.attach(&directories_label, 0, 0, 3, 1);

        let directories_container = gtk::Box::builder()
            .homogeneous(false)
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();

        let directories_view = StringList::new();
        directories_container.append(&scrolled(&directories_view.get_widget(), true));

        let directories_buttons = button_column(&[
            add_directory_button(&directories_view),
            remove_selection_button(&directories_view),
            clear_button(&directories_view),
        ]);
        directories_container.append(&directories_buttons);

        container.attach(&directories_container, 0, 1, 3, 1);

        let excluded_label = form_label("Paths to exclude");
        container.attach(&excluded_label, 0, 2, 3, 1);

        let excluded_view = StringList::new();
        let excluded = scrolled(&excluded_view.get_widget(), true);
        container.attach(&excluded, 0, 3, 2, 1);

        let excluded_buttons = button_column(&[
            add_excluded_directory_button(&excluded_view),
            add_exclusion_pattern_button(&excluded_view),
            remove_selection_button(&excluded_view),
            clear_button(&excluded_view),
        ]);
        container.attach(&excluded_buttons, 2, 3, 1, 1);

        let recurse = gtk::CheckButton::builder()
            .label("recurse?")
            .active(true)
            .build();

        container.attach(&recurse, 0, 4, 3, 1);

        let min_size_label = form_label("Minimum file size:");
        container.attach(&min_size_label, 0, 5, 1, 1);

        let min_size = gtk::Entry::builder()
            .tooltip_text("Using find -size syntax")
            .text("1")
            .hexpand(true)
            .vexpand(false)
            .build();
        container.attach(&min_size, 1, 5, 2, 1);

        // artificial expander for the column #1
        container.attach(&horizontal_expander(), 1, 100, 1, 1);

        Self {
            container,
            directories: directories_view,
            excluded: excluded_view,
            recurse,
            min_size,
        }
    }

    pub fn get_widget(&self) -> gtk::Widget {
        self.container.clone().upcast()
    }

    pub fn add_directory(&self, value: &Path) {
        self.directories.append(Directory(value.to_owned()))
    }

    pub fn add_excluded(&self, value: Exclusion) {
        self.excluded.append(value)
    }

    pub fn get_directories(&self) -> Vec<PathBuf> {
        self.directories.to_vec().into_iter().map(|d| d.0).collect()
    }

    pub fn get_excluded(&self) -> Vec<Exclusion> {
        self.excluded.to_vec()
    }

    pub fn get_recurse(&self) -> bool {
        self.recurse.is_active()
    }

    pub fn get_min_size(&self) -> u64 {
        self.min_size.text().parse::<u64>().unwrap_or_default()
    }
}
