use crate::duplicates_list;
use crate::errors;
use crate::find_duplicates::{duplication_status, find_duplicate_groups};
use crate::options;
use crate::path_choose;
use crate::user_interaction;
use crate::utils::horizontal_expander;
use crate::widgets::go_button::go_button;
use crate::widgets::menu_builder::MenuBuilderExt;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use std::cell::Cell;
use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

fn xdg_open(file: &Path) -> Result<(), Box<dyn Error>> {
    Command::new("xdg-open").arg(file).spawn()?;
    Ok(())
}

const DEFAULT_EXCLUDE_PATTERNS: &[&str] = &[
    "/lost+found",
    "/dev",
    "/proc",
    "/sys",
    "/tmp",
    "*/.svn",
    "*/CVS",
    "*/.git",
    "*/.hg",
    "*/.bzr",
    "*/node_modules",
    "*/target",
];

pub enum GroupCleanOption {
    First,
    Newest,
    Oldest,
}

fn all_buttons(builder: &mut AppWidgetsBuilder) -> gtk::Widget {
    let row = gtk::ButtonBoxBuilder::new()
        .homogeneous(false)
        .spacing(8)
        .margin_start(8)
        .margin_end(8)
        .orientation(gtk::Orientation::Horizontal)
        .layout_style(gtk::ButtonBoxStyle::End)
        .build();

    let del = gtk::ButtonBuilder::new().label("Delete").build();
    row.pack_end(&del, false, false, 1);

    let save = gtk::ButtonBuilder::new()
        .label("Save")
        .tooltip_text("Save (selected) list to file")
        .build();
    row.pack_end(&save, false, false, 1);

    let menu = gio::Menu::new()
        .item("Select using wildcard", "win.select_wildcard")
        .item("Unselect using wildcard", "win.unselect_wildcard")
        .submenu(
            "Select within groups",
            gio::Menu::new()
                .item("Select all but first", "win.select_but_first")
                .item("Select all but newest", "win.select_but_newest")
                .item("Select all but oldest", "win.select_but_oldest"),
        )
        .item("Toggle selection", "win.select_toggle")
        .item("Unselect all", "win.unselect_all");

    let select = gtk::MenuButtonBuilder::new()
        .label("Select")
        .menu_model(&menu)
        .use_popover(false)
        .direction(gtk::ArrowType::Up)
        .build();
    row.pack_end(&select, false, false, 1);

    builder.button_delete(del).button_save(save);

    row.upcast()
}

fn parameters(builder: &mut AppWidgetsBuilder) -> gtk::Widget {
    let b = gtk::GridBuilder::new()
        .column_homogeneous(false)
        .row_homogeneous(false)
        .row_spacing(16)
        .margin(8)
        .build();

    let options = options::Options::new();
    b.attach(&options.get_widget(), 0, 0, 2, 1);

    b.attach(&horizontal_expander(), 0, 1, 1, 1);

    let find = go_button("Find");
    b.attach(&find, 1, 1, 1, 1);

    builder.options(options).button_find(find);

    b.upcast()
}

fn results(builder: &mut AppWidgetsBuilder) -> gtk::Box {
    let b = gtk::BoxBuilder::new()
        .orientation(gtk::Orientation::Vertical)
        .homogeneous(false)
        .spacing(8)
        .build();

    let menu = gio::Menu::new()
        .item("Open", "win.open")
        .item("Open directory", "win.open_directory")
        .item("Copy", "win.copy")
        .item("Rename...", "win.rename")
        .item(
            "Select all in this directory",
            "win.select_from_same_folder",
        );

    let dups = duplicates_list::DuplicatesList::new(builder.duplicates.as_ref().unwrap());
    dups.set_popup(&menu.upcast());
    b.pack_start(&dups.get_widget(), true, true, 0);

    let all_buttons = all_buttons(builder);
    b.pack_start(&all_buttons, false, false, 0);

    let errors = errors::Errors::new();
    b.pack_start(&errors.get_widget(), false, false, 0);

    let status = gtk::Statusbar::new();
    b.pack_start(&status, false, false, 0);

    builder.view(dups).errors(errors).status(status);

    b
}

fn create_app_window(
    application: &gtk::Application,
    builder: &mut AppWidgetsBuilder,
) -> MainWindow {
    let window = gtk::ApplicationWindowBuilder::new()
        .application(application)
        .type_(gtk::WindowType::Toplevel)
        .default_width(1200)
        .default_height(800)
        .resizable(true)
        .build();

    let headerbar = gtk::HeaderBarBuilder::new()
        .show_close_button(true)
        .title("Find duplicates")
        .build();
    window.set_titlebar(Some(&headerbar));

    let paned = gtk::PanedBuilder::new().build();

    paned.pack1(&parameters(builder), false, false);
    paned.pack2(&results(builder), true, false);

    window.add(&paned);

    MainWindow(window)
}

#[derive(derive_builder::Builder)]
struct AppWidgets {
    duplicates: duplicates_list::DuplicatesStore,

    options: options::Options,

    button_find: gtk::Button,
    button_delete: gtk::Button,
    button_save: gtk::Button,

    view: duplicates_list::DuplicatesList,
    errors: errors::Errors,
    status: gtk::Statusbar,
}

#[derive(newtype_gobject::NewTypeGObject)]
pub struct MainWindow(pub gtk::ApplicationWindow);

pub struct MainWindowPrivate {
    confirm_delete: Cell<bool>,
    widgets: AppWidgets,
}

impl MainWindow {
    pub fn new(application: &gtk::Application) -> MainWindow {
        let mut widgets_builder = AppWidgetsBuilder::default();
        widgets_builder.duplicates(duplicates_list::DuplicatesStore::new());

        let window = create_app_window(application, &mut widgets_builder);

        let private = MainWindowPrivate {
            confirm_delete: Cell::new(true),
            widgets: widgets_builder.build().unwrap(),
        };
        Self::private_field().set(&window.0, private);

        window.connect_signals();

        for ignore in DEFAULT_EXCLUDE_PATTERNS {
            window.add_excluded(&ignore);
        }

        window
    }

    fn private_field() -> newtype_gobject::DynamicProperty<MainWindowPrivate> {
        newtype_gobject::DynamicProperty::new("private")
    }

    fn get_private(&self) -> &MainWindowPrivate {
        Self::private_field().get(&self.0).unwrap()
    }

    fn connect_signals(&self) {
        let private = self.get_private();
        private.widgets.button_find.connect_clicked(
            clone!(@weak self as window => move |_| window.fallible(window.on_find())),
        );
        private
            .widgets
            .button_delete
            .connect_clicked(clone!(@weak self as window => move |_| window.on_delete_selected()));
        private
            .widgets
            .button_save
            .connect_clicked(clone!(@weak self as window => move |_| window.on_save_as().unwrap()));

        self.create_action("open").connect_activate(
            clone!(@weak self as window => move |_, _| window.fallible(window.on_open_file())),
        );
        self.create_action("open_directory").connect_activate(
            clone!(@weak self as window => move |_, _| window.fallible(window.on_open_directory())),
        );
        self.create_action("copy").connect_activate(
            clone!(@weak self as window => move |_, _| window.on_copy_to_clipboard()),
        );
        self.create_action("rename").connect_activate(
            clone!(@weak self as window => move |_, _| window.fallible(window.on_rename())),
        );
        self.create_action("select_from_same_folder")
            .connect_activate(
                clone!(@weak self as window => move |_, _| window.on_select_from_that_folder()),
            );

        self.create_action("select_wildcard").connect_activate(
            clone!(@weak self as window => move |_, _| window.on_select_using_wildcard()),
        );
        self.create_action("unselect_wildcard").connect_activate(
            clone!(@weak self as window => move |_, _| window.on_unselect_using_wildcard()),
        );
        self.create_action("select_but_first").connect_activate(clone!(@weak self as window => move |_, _| window.on_select_all_but_one_in_each_group(GroupCleanOption::First)));
        self.create_action("select_but_newest").connect_activate(clone!(@weak self as window => move |_, _| window.on_select_all_but_one_in_each_group(GroupCleanOption::Newest)));
        self.create_action("select_but_oldest").connect_activate(clone!(@weak self as window => move |_, _| window.on_select_all_but_one_in_each_group(GroupCleanOption::Oldest)));
        self.create_action("select_toggle").connect_activate(
            clone!(@weak self as window => move |_, _| window.on_toggle_selection()),
        );
        self.create_action("unselect_all")
            .connect_activate(clone!(@weak self as window => move |_, _| window.on_unselect_all()));
    }

    fn create_action(&self, name: &str) -> gio::SimpleAction {
        let action = gio::SimpleAction::new(name, None);
        self.0.add_action(&action);
        action
    }

    pub fn add_directory(&self, directory: &str) {
        let private = self.get_private();
        private.widgets.options.add_directory(directory);
    }

    fn set_status(&self, message: &str) {
        let private = self.get_private();
        private.widgets.status.pop(0);
        private.widgets.status.push(0, message);
    }

    fn add_excluded(&self, excluded: &str) {
        let private = self.get_private();
        private.widgets.options.add_excluded(excluded);
    }

    fn fallible(&self, result: Result<(), Box<dyn Error>>) {
        if let Err(error) = result {
            self.show_error(&error.to_string());
        }
    }

    fn show_error(&self, line: &str) {
        let private = self.get_private();
        private.widgets.errors.append(line);
    }

    fn clear_errors(&self) {
        let private = self.get_private();
        private.widgets.errors.clear();
    }

    fn on_find(&self) -> Result<(), Box<dyn Error>> {
        let private = self.get_private();

        let search_dirs = private.widgets.options.get_directories();
        if search_dirs.is_empty() {
            return Err("No search paths specified".into());
        }
        let excluded = private.widgets.options.get_excluded();

        let min_size: u64 = private.widgets.options.get_min_size();
        let recurse = private.widgets.options.get_recurse();

        self.clear_errors();
        self.set_status("");
        private.widgets.duplicates.clear();

        self.set_status("searching...");
        let duplicates = find_duplicate_groups(&search_dirs, &excluded, min_size, recurse)?;
        self.set_status("processing...");

        for group in &duplicates {
            private
                .widgets
                .duplicates
                .append_group(group.files.len(), group.size());
            for fi in &group.files {
                private
                    .widgets
                    .duplicates
                    .append_file(&fi.path, fi.modified, fi.size);
            }
        }

        self.set_status("Done");

        let status = duplication_status(&duplicates);

        user_interaction::notify_info(&self.0.clone().upcast(), &status);

        Ok(())
    }

    fn on_save_as(&self) -> Result<(), Box<dyn Error>> {
        let private = self.get_private();
        let (selected, _model) = private.widgets.view.get_selection().get_selected_rows();
        let to_save = if selected.len() > 0 {
            self.get_selected_paths()
        } else {
            private
                .widgets
                .duplicates
                .group_iter()
                .flat_map(|(_group, files)| files.into_iter())
                .filter_map(|iter| private.widgets.duplicates.get_path(&iter))
                .collect()
        };

        if to_save.is_empty() {
            return Ok(());
        }

        let pwd = env::current_dir().unwrap();
        let file_save_as = match path_choose::save_as(&self.0.clone().upcast(), &pwd) {
            Some(path) => path,
            None => return Ok(()),
        };

        if file_save_as.exists() {
            if file_save_as.is_file() {
                if !user_interaction::confirm(
                    &self.0.clone().upcast(),
                    &format!("Do you want to overwrite?\n{}", file_save_as.display()),
                ) {
                    return Ok(());
                }
            } else {
                user_interaction::notify_error(
                    &self.0.clone().upcast(),
                    &format!("You can't overwrite {}", file_save_as.display()),
                );
                return Ok(());
            }
        }

        self.clear_errors();

        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_save_as)?;

        for path in to_save {
            file.write_all(path.to_str().unwrap().as_bytes())?;
            file.write_all(b"\n")?;
        }
        Ok(())
    }

    fn get_selected_iter(&self) -> Option<gtk::TreeIter> {
        let private = self.get_private();
        let (selected, model) = private.widgets.view.get_selection().get_selected_rows();
        if selected.len() != 1 {
            return None;
        }
        let iter = model.get_iter(&selected[0])?;
        Some(iter)
    }

    fn get_selected_path(&self) -> Option<PathBuf> {
        let iter = self.get_selected_iter()?;
        let private = self.get_private();
        private.widgets.duplicates.get_path(&iter)
    }

    fn get_selected_paths(&self) -> Vec<PathBuf> {
        let private = self.get_private();
        let (selected, model) = private.widgets.view.get_selection().get_selected_rows();
        selected
            .into_iter()
            .filter_map(|tree_path| model.get_iter(&tree_path))
            .filter_map(|iter| private.widgets.duplicates.get_path(&iter))
            .collect()
    }

    fn on_copy_to_clipboard(&self) {
        if let Some(path) = self.get_selected_path() {
            let clipboard = gtk::Clipboard::get(&gdk::Atom::intern("CLIPBOARD"));
            clipboard.set_text(path.to_str().unwrap());
        }
    }

    fn on_open_file(&self) -> Result<(), Box<dyn Error>> {
        if let Some(path) = self.get_selected_path() {
            xdg_open(&path)?;
        }
        Ok(())
    }

    fn on_open_directory(&self) -> Result<(), Box<dyn Error>> {
        if let Some(dir) = self
            .get_selected_path()
            .and_then(|path| path.parent().map(|p| p.to_path_buf()))
        {
            xdg_open(&dir)?;
        }
        Ok(())
    }

    // select all other duplicates from selected item folder
    fn on_select_from_that_folder(&self) {
        let private = self.get_private();
        if let Some(path) = self.get_selected_path() {
            if let Some(dir) = path.parent() {
                for (_group, files) in private.widgets.duplicates.group_iter() {
                    for file in files {
                        if private
                            .widgets
                            .duplicates
                            .get_path(&file)
                            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                            == Some(dir.to_path_buf())
                        {
                            private.widgets.view.get_selection().select_iter(&file);
                        }
                    }
                }
            }
        }
    }

    fn get_path_and_name(&self, iter: &gtk::TreeIter) -> Result<(PathBuf, String), Box<dyn Error>> {
        let private = self.get_private();
        let path = private
            .widgets
            .duplicates
            .get_path(&iter)
            .ok_or_else(|| "Cannot get path of the file")?;
        let name = path
            .file_name()
            .ok_or_else(|| "Cannot get base name of the file")?
            .to_str()
            .ok_or_else(|| "Cannot convert file name to string")?
            .to_owned();
        Ok((path, name))
    }

    fn on_rename(&self) -> Result<(), Box<dyn Error>> {
        let iter = match self.get_selected_iter() {
            Some(iter) => iter,
            None => return Ok(()),
        };

        let (old_path, old_name) = self.get_path_and_name(&iter)?;

        let new_name = match user_interaction::prompt(
            &self.0.clone().upcast(),
            "Rename file",
            "Name:",
            &old_name,
        ) {
            Some(name) if name != old_name => name,
            _ => return Ok(()),
        };

        self.clear_errors();

        let mut new_path = old_path.parent().unwrap().to_path_buf();
        new_path.push(new_name);

        if new_path.exists() {
            return Err(format!(
                "Error: Can't rename [{}] as [{}] exists",
                old_name,
                new_path.display()
            )
            .into());
        }

        fs::rename(old_path, &new_path)?;

        let private = self.get_private();
        private.widgets.duplicates.set_path(&iter, &new_path);

        Ok(())
    }

    fn on_unselect_using_wildcard(&self) {
        self.fallible(self.select_using_wildcard(false));
    }

    fn on_select_using_wildcard(&self) {
        self.fallible(self.select_using_wildcard(true));
    }

    fn select_using_wildcard(&self, select: bool) -> Result<(), Box<dyn Error>> {
        let private = self.get_private();
        if private.widgets.duplicates.is_empty() {
            return Ok(());
        }

        let title = if select {
            "Select by wildcard"
        } else {
            "Unselect by wildcard"
        };
        let wildcard =
            match user_interaction::prompt(&self.0.clone().upcast(), title, "Wildcard:", "*") {
                Some(answer) if !answer.is_empty() => answer,
                _ => return Ok(()),
            };

        self.clear_errors();
        let pattern = glob::Pattern::new(&wildcard)?;
        let selection = private.widgets.view.get_selection();
        for (_group, files) in private.widgets.duplicates.group_iter() {
            for file_iter in files {
                let fs_path = private.widgets.duplicates.get_path(&file_iter).unwrap();
                if pattern.matches_path(&fs_path) {
                    if select {
                        selection.select_iter(&file_iter);
                    } else {
                        selection.unselect_iter(&file_iter);
                    }
                }
            }
        }
        Ok(())
    }

    fn on_select_all_but_one_in_each_group(&self, which: GroupCleanOption) {
        fn find_row_to_unselect<'i>(
            clist: &duplicates_list::DuplicatesStore,
            files: &'i [gtk::TreeIter],
            which: &GroupCleanOption,
        ) -> Option<&'i gtk::TreeIter> {
            match which {
                GroupCleanOption::First => files.first(),
                GroupCleanOption::Newest => files.iter().max_by_key(|iter| clist.modified(iter)),
                GroupCleanOption::Oldest => files.iter().min_by_key(|iter| clist.modified(iter)),
            }
        }

        let private = self.get_private();
        let selection = private.widgets.view.get_selection();
        for (_group, files) in private.widgets.duplicates.group_iter() {
            for file in &files {
                selection.select_iter(file);
            }
            if let Some(unselect) =
                find_row_to_unselect(&private.widgets.duplicates, &files, &which)
            {
                selection.unselect_iter(unselect);
            }
        }
    }

    fn on_unselect_all(&self) {
        let private = self.get_private();
        private.widgets.view.get_selection().unselect_all();
    }

    fn on_toggle_selection(&self) {
        let private = self.get_private();

        let selection = private.widgets.view.get_selection();
        for iter in private.widgets.duplicates.iter() {
            if !private.widgets.duplicates.is_group(&iter) {
                if selection.iter_is_selected(&iter) {
                    selection.unselect_iter(&iter);
                } else {
                    selection.select_iter(&iter);
                }
            }
        }
    }

    fn on_delete_selected(&self) {
        let private = self.get_private();
        let (selected, _model) = private.widgets.view.get_selection().get_selected_rows();

        self.clear_errors();
        self.set_status("");

        let count = selected.len();
        if count == 0 {
            self.show_error("None selected");
            return;
        }

        if self.get_private().confirm_delete.get() {
            let question = if count == 1 {
                "Are you sure you want to delete this file?".into()
            } else {
                format!("Are you sure you want to delete these {} files?", count)
            };
            let (confirm, ask_again) =
                user_interaction::confirm_delete(&self.0.clone().upcast(), &question);
            self.get_private().confirm_delete.set(ask_again);
            if !confirm {
                return;
            }
        }

        let mut deleted: Vec<gtk::TreeIter> = Vec::new();
        for tree_path in selected {
            match self.delete_by_tree_path(&tree_path) {
                Ok(iter) => {
                    deleted.push(iter);
                }
                Err(error) => {
                    self.show_error(&error.to_string());
                }
            }
        }

        private.widgets.duplicates.remove_all(&deleted);
        private
            .widgets
            .duplicates
            .remove_groups_without_duplications();
        self.set_status(&format!("{} items deleted", deleted.len()));
    }

    fn delete_by_tree_path(&self, tree_path: &gtk::TreePath) -> io::Result<gtk::TreeIter> {
        let private = self.get_private();
        let iter = private
            .widgets
            .duplicates
            .to_model()
            .get_iter(tree_path)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Cannot get iter of tree path."))?;
        let fs_path = private.widgets.duplicates.get_path(&iter).ok_or_else(|| {
            io::Error::new(io::ErrorKind::Other, "Cannot get path to file by iter.")
        })?;
        fs::remove_file(&fs_path)?;
        Ok(iter)
    }
}
