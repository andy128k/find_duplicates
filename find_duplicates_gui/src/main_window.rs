use crate::action_name::ActionName;
use crate::duplicates_list;
use crate::exclusion::{Exclusion, DEFAULT_EXCLUDE_PATTERNS};
use crate::find_duplicates::{duplication_status, find_duplicate_groups, DuplicatesGroup};
use crate::options;
use crate::path_choose;
use crate::user_interaction;
use crate::utils::horizontal_expander;
use crate::widgets::go_button::go_button;
use crate::widgets::menu_builder::MenuBuilderExt;
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use newtype_gobject::object_data::ObjectDataExt;
use std::cell::{Cell, RefCell};
use std::env;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

mod action {
    use crate::action_name::*;

    pub const FIND: ActionName = action_name!(win, find);

    pub const SELECT_WILDCARD: ActionName = action_name!(win, select_wildcard);
    pub const UNSELECT_WILDCARD: ActionName = action_name!(win, unselect_wildcard);

    pub const SELECT_ALL_BUT_FIRST: ActionName = action_name!(win, select_but_first);
    pub const SELECT_ALL_BUT_NEWEST: ActionName = action_name!(win, select_but_newest);
    pub const SELECT_ALL_BUT_OLDEST: ActionName = action_name!(win, select_but_oldest);

    pub const SELECT_TOGGLE: ActionName = action_name!(win, select_toggle);
    pub const UNSELECT_ALL: ActionName = action_name!(win, unselect_all);

    pub const OPEN: ActionName = action_name!(win, open);
    pub const OPEN_DIRECTORY: ActionName = action_name!(win, open_directory);
    pub const COPY: ActionName = action_name!(win, copy);
    pub const RENAME: ActionName = action_name!(win, rename);
    pub const SELECT_FROM_SAME_FOLDER: ActionName = action_name!(win, select_from_same_folder);

    pub const DELETE: ActionName = action_name!(win, delete);
    pub const SAVE: ActionName = action_name!(win, save);
}

fn xdg_open(file: &Path) -> Result<(), Box<dyn Error>> {
    Command::new("xdg-open").arg(file).spawn()?;
    Ok(())
}

pub enum GroupCleanOption {
    First,
    Newest,
    Oldest,
}

fn action_buttons() -> gtk::Widget {
    let row = gtk::ButtonBoxBuilder::new()
        .homogeneous(false)
        .spacing(8)
        .margin(8)
        .orientation(gtk::Orientation::Horizontal)
        .layout_style(gtk::ButtonBoxStyle::End)
        .build();

    let del = gtk::ButtonBuilder::new()
        .label("Delete")
        .action_name(action::DELETE.full())
        .build();
    row.pack_end(&del, false, false, 1);

    let save = gtk::ButtonBuilder::new()
        .label("Save")
        .tooltip_text("Save (selected) list to file")
        .action_name(action::SAVE.full())
        .build();
    row.pack_end(&save, false, false, 1);

    let menu = gio::Menu::new()
        .item("Select using wildcard", &action::SELECT_WILDCARD)
        .item("Unselect using wildcard", &action::UNSELECT_WILDCARD)
        .submenu(
            "Select within groups",
            gio::Menu::new()
                .item("Select all but first", &action::SELECT_ALL_BUT_FIRST)
                .item("Select all but newest", &action::SELECT_ALL_BUT_NEWEST)
                .item("Select all but oldest", &action::SELECT_ALL_BUT_OLDEST),
        )
        .item("Toggle selection", &action::SELECT_TOGGLE)
        .item("Unselect all", &action::UNSELECT_ALL);

    let select = gtk::MenuButtonBuilder::new()
        .label("Select")
        .menu_model(&menu)
        .use_popover(false)
        .direction(gtk::ArrowType::Up)
        .build();
    row.pack_end(&select, false, false, 1);

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
    find.set_action_name(Some(action::FIND.full()));
    b.attach(&find, 1, 1, 1, 1);

    builder.options(options);

    b.upcast()
}

fn results(builder: &mut AppWidgetsBuilder) -> gtk::Box {
    let b = gtk::BoxBuilder::new()
        .orientation(gtk::Orientation::Vertical)
        .homogeneous(false)
        .build();

    let menu = gio::Menu::new()
        .item("Open", &action::OPEN)
        .item("Open directory", &action::OPEN_DIRECTORY)
        .item("Copy", &action::COPY)
        .item("Rename...", &action::RENAME)
        .item(
            "Select all in this directory",
            &action::SELECT_FROM_SAME_FOLDER,
        );

    let dups = duplicates_list::DuplicatesList::new(builder.duplicates.as_ref().unwrap());
    dups.set_popup(&menu.upcast());
    b.pack_start(&dups.get_widget(), true, true, 0);

    let all_buttons = action_buttons();
    b.pack_start(&all_buttons, false, false, 0);

    builder.view(dups);

    b
}

fn create_app_window(
    application: &gtk::Application,
    builder: &mut AppWidgetsBuilder,
) -> MainWindow {
    let window = gtk::ApplicationWindowBuilder::new()
        .application(application)
        .type_(gtk::WindowType::Toplevel)
        .window_position(gtk::WindowPosition::Center)
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

type FindResult = Result<Vec<DuplicatesGroup>, String>;

#[derive(derive_builder::Builder)]
struct AppWidgets {
    duplicates: duplicates_list::DuplicatesStore,
    options: options::Options,
    view: duplicates_list::DuplicatesList,
}

#[derive(newtype_gobject::NewTypeGObject)]
pub struct MainWindow(pub gtk::ApplicationWindow);

pub struct MainWindowPrivate {
    confirm_delete: Cell<bool>,
    widgets: AppWidgets,

    find_sender: glib::Sender<FindResult>,
    progress: RefCell<Option<gtk::Dialog>>,
}

impl MainWindow {
    pub fn new(application: &gtk::Application) -> MainWindow {
        let mut widgets_builder = AppWidgetsBuilder::default();
        widgets_builder.duplicates(duplicates_list::DuplicatesStore::new());

        let window = create_app_window(application, &mut widgets_builder);

        let (find_sender, find_receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let private = MainWindowPrivate {
            confirm_delete: Cell::new(true),
            widgets: widgets_builder.build().unwrap(),
            find_sender,
            progress: RefCell::new(None),
        };
        unsafe {
            window.0.set_data::<MainWindowPrivate>("private", private);
        }

        window.connect_signals(find_receiver);

        for ignore in DEFAULT_EXCLUDE_PATTERNS.iter() {
            window.add_excluded(ignore.clone());
        }

        window
    }

    fn get_private(&self) -> &MainWindowPrivate {
        unsafe { self.0.get_data::<MainWindowPrivate>("private").unwrap() }
    }

    fn connect_signals(&self, find_receiver: glib::Receiver<FindResult>) {
        self.create_action(&action::FIND).connect_activate(
            clone!(@weak self as window => move |_, _| window.fallible(window.on_find())),
        );
        self.create_action(&action::DELETE).connect_activate(
            clone!(@weak self as window => move |_, _| window.on_delete_selected()),
        );
        self.create_action(&action::SAVE).connect_activate(
            clone!(@weak self as window => move |_, _| window.on_save_as().unwrap()),
        );
        self.create_action(&action::OPEN).connect_activate(
            clone!(@weak self as window => move |_, _| window.fallible(window.on_open_file())),
        );
        self.create_action(&action::OPEN_DIRECTORY).connect_activate(
            clone!(@weak self as window => move |_, _| window.fallible(window.on_open_directory())),
        );
        self.create_action(&action::COPY).connect_activate(
            clone!(@weak self as window => move |_, _| window.on_copy_to_clipboard()),
        );
        self.create_action(&action::RENAME).connect_activate(
            clone!(@weak self as window => move |_, _| window.fallible(window.on_rename())),
        );
        self.create_action(&action::SELECT_FROM_SAME_FOLDER)
            .connect_activate(
                clone!(@weak self as window => move |_, _| window.on_select_from_that_folder()),
            );

        self.create_action(&action::SELECT_WILDCARD)
            .connect_activate(
                clone!(@weak self as window => move |_, _| window.fallible(window.on_select_using_wildcard(true))),
            );
        self.create_action(&action::UNSELECT_WILDCARD)
            .connect_activate(
                clone!(@weak self as window => move |_, _| window.fallible(window.on_select_using_wildcard(false))),
            );
        self.create_action(&action::SELECT_ALL_BUT_FIRST).connect_activate(clone!(@weak self as window => move |_, _| window.on_select_all_but_one_in_each_group(GroupCleanOption::First)));
        self.create_action(&action::SELECT_ALL_BUT_NEWEST).connect_activate(clone!(@weak self as window => move |_, _| window.on_select_all_but_one_in_each_group(GroupCleanOption::Newest)));
        self.create_action(&action::SELECT_ALL_BUT_OLDEST).connect_activate(clone!(@weak self as window => move |_, _| window.on_select_all_but_one_in_each_group(GroupCleanOption::Oldest)));
        self.create_action(&action::SELECT_TOGGLE).connect_activate(
            clone!(@weak self as window => move |_, _| window.on_toggle_selection()),
        );
        self.create_action(&action::UNSELECT_ALL)
            .connect_activate(clone!(@weak self as window => move |_, _| window.on_unselect_all()));

        find_receiver.attach(None, clone!(@weak self as window =>  @default-return glib::Continue(false), move |msg| window.on_find_finished(msg)));
    }

    fn create_action(&self, name: &ActionName) -> gio::SimpleAction {
        let action = gio::SimpleAction::new(name.local(), None);
        self.0.add_action(&action);
        action
    }

    pub fn add_directory(&self, directory: &Path) {
        let private = self.get_private();
        private.widgets.options.add_directory(directory);
    }

    fn add_excluded(&self, excluded: Exclusion) {
        let private = self.get_private();
        private.widgets.options.add_excluded(excluded);
    }

    fn fallible(&self, result: Result<(), Box<dyn Error>>) {
        if let Err(error) = result {
            user_interaction::notify_error(&self.0.clone().upcast(), &error.to_string());
        }
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

        private.widgets.duplicates.clear();

        let progress = user_interaction::progress(&self.0.clone().upcast(), "Searching...");
        progress.show();
        *private.progress.borrow_mut() = Some(progress);

        let sender = private.find_sender.clone();
        thread::spawn(move || {
            let duplicates = find_duplicate_groups(&search_dirs, &excluded, min_size, recurse);
            let _ = sender.send(duplicates.map_err(|err| err.to_string()));
        });
        Ok(())
    }

    fn on_find_finished(&self, msg: FindResult) -> glib::Continue {
        let private = self.get_private();

        if let Some(progress) = private.progress.borrow_mut().take() {
            progress.destroy();
        }

        match msg {
            Ok(duplicates) => {
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

                let status = duplication_status(&duplicates);

                user_interaction::notify_info(&self.0.clone().upcast(), &status);
            }
            Err(error) => {
                user_interaction::notify_error(&self.0.clone().upcast(), &error);
            }
        }
        glib::Continue(true)
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
                .filter_map(|iter| private.widgets.duplicates.get_fs_path(&iter))
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
        private.widgets.duplicates.get_fs_path(&iter)
    }

    fn get_selected_paths(&self) -> Vec<PathBuf> {
        let private = self.get_private();
        let (selected, model) = private.widgets.view.get_selection().get_selected_rows();
        selected
            .into_iter()
            .filter_map(|tree_path| model.get_iter(&tree_path))
            .filter_map(|iter| private.widgets.duplicates.get_fs_path(&iter))
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
                            .get_fs_path(&file)
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
            .get_fs_path(&iter)
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

    fn on_select_using_wildcard(&self, select: bool) -> Result<(), Box<dyn Error>> {
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

        let pattern = glob::Pattern::new(&wildcard)?;
        let selection = private.widgets.view.get_selection();
        for (_group, files) in private.widgets.duplicates.group_iter() {
            for file_iter in files {
                let fs_path = private.widgets.duplicates.get_fs_path(&file_iter).unwrap();
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
            model: &duplicates_list::DuplicatesStore,
            files: &'i [gtk::TreeIter],
            which: &GroupCleanOption,
        ) -> Option<&'i gtk::TreeIter> {
            match which {
                GroupCleanOption::First => files.first(),
                GroupCleanOption::Newest => files.iter().max_by_key(|iter| model.modified(iter)),
                GroupCleanOption::Oldest => files.iter().min_by_key(|iter| model.modified(iter)),
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

        let count = selected.len();
        if count == 0 {
            user_interaction::notify_error(&self.0.clone().upcast(), "No file is selected");
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
        let mut errors = Vec::new();
        for tree_path in selected {
            match self.delete_by_tree_path(&tree_path) {
                Ok(iter) => {
                    deleted.push(iter);
                }
                Err(error) => {
                    errors.push(error);
                }
            }
        }

        private.widgets.duplicates.remove_all(&deleted);

        if errors.is_empty() {
            user_interaction::notify_info(
                &self.0.clone().upcast(),
                &format!("{} items deleted", deleted.len()),
            );
        } else {
            let mut error_message = String::from("Following errors happened:\n");
            for error in errors {
                error_message.push_str("\n");
                error_message.push_str(&error.to_string());
            }
            user_interaction::notify_detailed(
                &self.0.clone().upcast(),
                &format!("{} items deleted", deleted.len()),
                &error_message,
            );
        }
    }

    fn delete_by_tree_path(
        &self,
        tree_path: &gtk::TreePath,
    ) -> Result<gtk::TreeIter, Box<dyn Error>> {
        let private = self.get_private();
        let iter = private
            .widgets
            .duplicates
            .to_model()
            .get_iter(tree_path)
            .ok_or_else(|| "Cannot get iter of tree path.")?;
        let fs_path = private
            .widgets
            .duplicates
            .get_fs_path(&iter)
            .ok_or_else(|| "Cannot get path to file by iter.")?;
        fs::remove_file(&fs_path)
            .map_err(|e| format!("File {} cannot be removed. {}", fs_path.display(), e))?;
        Ok(iter)
    }
}
