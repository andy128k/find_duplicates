use crate::exclusion::Exclusion;
use crate::path_choose::select_dir;
use crate::string_list::StringList;
use crate::user_interaction::prompt;
use crate::utils::horizontal_expander;
use glib::{clone, IsA};
use gtk::prelude::*;
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
    gtk::LabelBuilder::new()
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
    widget
        .get_toplevel()
        .and_then(|w| w.downcast::<gtk::Window>().ok())
}

fn pick_directory(widget: &impl IsA<gtk::Widget>) -> Option<PathBuf> {
    let window = get_window(widget)?;
    let pwd = std::env::current_dir().ok()?;
    let path = select_dir(&window, &pwd)?;
    Some(path)
}

fn pick_pattern(widget: &impl IsA<gtk::Widget>) -> Option<String> {
    let window = get_window(widget)?;
    let pattern = prompt(&window, "Add pattern", "pattern", "")?;
    if pattern.is_empty() {
        None
    } else {
        Some(pattern)
    }
}

fn add_directory_button(string_list: &StringList<Directory>) -> gtk::Button {
    let button = gtk::ButtonBuilder::new()
        .label("Add directory")
        .hexpand(false)
        .build();
    button.connect_clicked(clone!(@weak string_list => move |button| {
        if let Some(new_value) = pick_directory(button) {
            string_list.append(Directory(new_value));
        }
    }));
    button
}

fn add_excluded_directory_button(string_list: &StringList<Exclusion>) -> gtk::Button {
    let button = gtk::ButtonBuilder::new()
        .label("Add directory")
        .hexpand(false)
        .build();
    button.connect_clicked(clone!(@weak string_list => move |button| {
        if let Some(new_value) = pick_directory(button) {
            string_list.append(Exclusion::Directory(new_value));
        }
    }));
    button
}

fn add_exclusion_pattern_button(string_list: &StringList<Exclusion>) -> gtk::Button {
    let button = gtk::ButtonBuilder::new()
        .label("Add pattern")
        .hexpand(false)
        .build();
    button.connect_clicked(clone!(@weak string_list => move |button| {
        if let Some(new_value) = pick_pattern(button) {
            string_list.append(Exclusion::Pattern(new_value));
        }
    }));
    button
}

fn remove_selection_button<T: 'static>(string_list: &StringList<T>) -> gtk::Button {
    let button = gtk::ButtonBuilder::new()
        .label("Remove")
        .hexpand(false)
        .build();
    button.connect_clicked(clone!(@weak string_list => move |_|
        string_list.remove_selection()
    ));
    button
}

fn clear_button<T: 'static>(string_list: &StringList<T>) -> gtk::Button {
    let button = gtk::ButtonBuilder::new()
        .label("Clear")
        .hexpand(false)
        .build();
    button.connect_clicked(clone!(@weak string_list => move |_|
        string_list.clear()
    ));
    button
}

fn button_column(buttons: &[gtk::Button]) -> gtk::Widget {
    let container = gtk::BoxBuilder::new()
        .homogeneous(false)
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .build();

    for button in buttons {
        container.pack_start(button, false, false, 0);
    }

    container.upcast()
}

impl Options {
    pub fn new() -> Self {
        let container = gtk::GridBuilder::new()
            .column_homogeneous(false)
            .row_homogeneous(false)
            .column_spacing(8)
            .row_spacing(8)
            .build();

        let directories_label = form_label("Directories to search");
        container.attach(&directories_label, 0, 0, 3, 1);

        let directories_container = gtk::BoxBuilder::new()
            .homogeneous(false)
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();

        let directories = StringList::new();
        directories_container.pack_start(&directories.get_widget(), true, true, 0);

        let directories_buttons = button_column(&[
            add_directory_button(&directories),
            remove_selection_button(&directories),
            clear_button(&directories),
        ]);
        directories_container.pack_start(&directories_buttons, false, false, 0);

        container.attach(&directories_container, 0, 1, 3, 1);

        let excluded_label = form_label("Paths to exclude");
        container.attach(&excluded_label, 0, 2, 3, 1);

        let excluded = StringList::new();
        container.attach(&excluded.get_widget(), 0, 3, 2, 1);

        let excluded_buttons = button_column(&[
            add_excluded_directory_button(&excluded),
            add_exclusion_pattern_button(&excluded),
            remove_selection_button(&excluded),
            clear_button(&excluded),
        ]);
        container.attach(&excluded_buttons, 2, 3, 1, 1);

        let recurse = gtk::CheckButtonBuilder::new()
            .label("recurse?")
            .active(true)
            .build();

        container.attach(&recurse, 0, 4, 3, 1);

        let min_size_label = form_label("Minimum file size:");
        container.attach(&min_size_label, 0, 5, 1, 1);

        let min_size = gtk::EntryBuilder::new()
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
            directories,
            excluded,
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
        self.recurse.get_active()
    }

    pub fn get_min_size(&self) -> u64 {
        self.min_size
            .get_text()
            .and_then(|text| u64::from_str_radix(&text, 10).ok())
            .unwrap_or_default()
    }
}
