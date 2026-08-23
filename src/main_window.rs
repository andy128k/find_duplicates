use crate::duplicate::Duplicate;
use crate::duplicates_list::{self, Duplicates};
use crate::exclusion::{DEFAULT_EXCLUDE_PATTERNS, Exclusion};
use crate::find_duplicates::{DuplicatesGroup, duplication_status, find_duplicate_groups};
use crate::options;
use crate::path_choose;
use crate::user_interaction::{self, ProgressDialog};
use crate::utils::horizontal_expander;
use crate::widgets::go_button::go_button;
use crate::widgets::menu_builder::MenuBuilderExt;
use gtk::{gio, glib, pango, prelude::*, subclass::prelude::*};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

fn xdg_open(file: &Path) -> Result<(), Box<dyn Error>> {
    Command::new("xdg-open").arg(file).spawn()?;
    Ok(())
}

enum ItemWithinGroup {
    First,
    Newest,
    Oldest,
}

fn action_buttons() -> gtk::Widget {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .homogeneous(false)
        .spacing(8)
        .margin_start(8)
        .margin_end(8)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    row.append(&gtk::Label::builder().hexpand(true).build());

    let menu = gio::Menu::new()
        .item("Select using wildcard", "win.select-wildcard(true)")
        .item("Unselect using wildcard", "win.select-wildcard(false)")
        .submenu(
            "Select within groups",
            gio::Menu::new()
                .item("Select all but first", r#"win.select-all-but("first")"#)
                .item("Select all but newest", r#"win.select-all-but("newest")"#)
                .item("Select all but oldest", r#"win.select-all-but("oldest")"#),
        )
        .item("Toggle selection", "win.select-toggle")
        .item("Unselect all", "win.unselect-all");

    let select = gtk::MenuButton::builder()
        .label("Select")
        .menu_model(&menu)
        .direction(gtk::ArrowType::Up)
        .build();
    row.append(&select);

    let save = gtk::Button::builder()
        .label("Save")
        .tooltip_text("Save (selected) list to file")
        .action_name("win.save")
        .build();
    row.append(&save);

    let del = gtk::Button::builder()
        .label("Delete")
        .action_name("win.delete")
        .build();
    row.append(&del);

    row.upcast()
}

fn sidebar_layout(child: &gtk::Widget, action: &gtk::Button) -> gtk::Widget {
    let grid = gtk::Grid::builder()
        .column_homogeneous(false)
        .row_homogeneous(false)
        .row_spacing(16)
        .margin_start(8)
        .margin_end(8)
        .margin_top(8)
        .margin_bottom(8)
        .build();
    grid.attach(child, 0, 0, 2, 1);
    grid.attach(&horizontal_expander(), 0, 1, 1, 1);
    grid.attach(action, 1, 1, 1, 1);
    grid.upcast()
}

fn results_layout(top: &gtk::Widget, bottom: &gtk::Widget) -> gtk::Widget {
    let bx = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .homogeneous(false)
        .build();
    top.set_vexpand(true);
    bx.append(top);
    bx.append(bottom);
    bx.upcast()
}

fn panes(sidebar: &gtk::Widget, main: &gtk::Widget) -> gtk::Widget {
    gtk::Paned::builder()
        .start_child(sidebar)
        .end_child(main)
        .resize_start_child(false)
        .shrink_start_child(false)
        .resize_end_child(true)
        .shrink_end_child(false)
        .build()
        .upcast()
}

fn duplicates_popup() -> gio::Menu {
    gio::Menu::new()
        .item("Open", "win.open")
        .item("Open directory", "win.open-directory")
        .item("Copy", "win.copy")
        .item("Rename...", "win.rename")
        .item(
            "Select all in this directory",
            "win.select-from-same-directory",
        )
}

type FindResult = Result<Vec<DuplicatesGroup>, String>;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct MainWindow {
        pub confirm_delete: Cell<bool>,
        pub duplicates: duplicates_list::DuplicatesStore,
        pub options: options::Options,
        pub view: duplicates_list::DuplicatesList,
        pub progress: RefCell<Option<ProgressDialog>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MainWindow {
        const NAME: &'static str = "MainWindow";
        type Type = super::MainWindow;
        type ParentType = gtk::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.install_action_async(
                "win.find",
                None,
                |win, _, _| async move { win.find().await },
            );
            klass.install_action_async(
                "win.open",
                None,
                |win, _, _| async move { win.open().await },
            );
            klass.install_action_async("win.open-directory", None, |win, _, _| async move {
                win.open_directory().await
            });
            klass.install_action("win.copy", None, |win, _, _| win.copy());
            klass.install_action_async("win.rename", None, |win, _, _| async move {
                win.rename().await
            });
            klass.install_action("win.select-from-same-directory", None, |win, _, _| {
                win.select_from_same_directory()
            });
            klass.install_action_async(
                "win.select-wildcard",
                Some(&bool::static_variant_type()),
                |win, _, p| async move {
                    let select = p.and_then(|v| v.get()) == Some(true);
                    win.select_wildcard(select).await;
                },
            );
            klass.install_action(
                "win.select-all-but",
                Some(&String::static_variant_type()),
                |win, _, p| {
                    if let Some(all_but) = p.and_then(|v| v.get()) {
                        win.select_all_but(all_but);
                    }
                },
            );
            klass.install_action("win.select-toggle", None, |win, _, _| win.select_toggle());
            klass.install_action("win.unselect-all", None, |win, _, _| win.unselect_all());
            klass.install_action_async("win.delete", None, |win, _, _| async move {
                win.delete().await
            });
            klass.install_action_async(
                "win.save",
                None,
                |win, _, _| async move { win.save().await },
            );
        }
    }

    impl ObjectImpl for MainWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let window = self.obj();
            window.set_default_width(1200);
            window.set_default_height(800);
            window.set_resizable(true);

            let title_label = gtk::Label::builder()
                .label("Find duplicates")
                .single_line_mode(true)
                .ellipsize(pango::EllipsizeMode::End)
                .build();
            title_label.add_css_class("title");

            let headerbar = gtk::HeaderBar::builder()
                .show_title_buttons(true)
                .title_widget(&title_label)
                .build();
            window.set_titlebar(Some(&headerbar));

            let menu = duplicates_popup();

            self.view.set_model(&self.duplicates);
            self.view.set_popup(&menu.upcast());

            let action_buttons = action_buttons();

            let paned = panes(
                &sidebar_layout(&self.options.get_widget(), &go_button("Find", "win.find")),
                &results_layout(self.view.upcast_ref(), &action_buttons),
            );

            window.set_child(Some(&paned));

            for ignore in DEFAULT_EXCLUDE_PATTERNS.iter() {
                window.add_excluded(ignore.clone());
            }

            self.confirm_delete.set(true);
        }
    }

    impl WidgetImpl for MainWindow {}
    impl WindowImpl for MainWindow {}
    impl ApplicationWindowImpl for MainWindow {}

    impl MainWindow {
        pub async fn on_find_finished(&self, msg: FindResult) {
            if let Some(progress) = self.progress.borrow_mut().take() {
                progress.close().await;
            }

            match msg {
                Ok(duplicates) => {
                    for group in &duplicates {
                        let gr = Duplicates::new();
                        for fi in &group.files {
                            gr.append_file(&fi.path, fi.modified, fi.size);
                        }
                        self.duplicates.append_group(gr);
                    }

                    let status = duplication_status(&duplicates);

                    user_interaction::notify_info(self.obj().upcast_ref(), &status).await;
                }
                Err(error) => {
                    self.show_error(&error).await;
                }
            }
        }

        async fn show_error(&self, message: impl ToString) {
            user_interaction::notify_error(self.obj().upcast_ref(), &message.to_string()).await;
        }
    }
}

glib::wrapper! {
    pub struct MainWindow(ObjectSubclass<imp::MainWindow>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::ShortcutManager, gtk::Accessible, gtk::Root, gtk::Native,
            gio::ActionGroup, gio::ActionMap;
}

impl MainWindow {
    pub fn new(application: &gtk::Application) -> Self {
        let window: Self = glib::Object::builder().build();
        window.set_application(Some(application));
        window
    }

    pub fn add_directory(&self, directory: &Path) {
        self.imp().options.add_directory(directory);
    }

    fn add_excluded(&self, excluded: Exclusion) {
        self.imp().options.add_excluded(excluded);
    }

    async fn do_save(&self) -> Result<(), Box<dyn Error>> {
        let private = self.imp();
        let selected = private.view.selected();
        let to_save: Vec<PathBuf> = if !selected.is_empty() {
            selected
                .iter()
                .map(|(_position, item)| item.path())
                .collect()
        } else {
            private
                .duplicates
                .group_iter()
                .flat_map(|group| {
                    group
                        .iter()
                        .map(|item| item.path())
                        .collect::<Vec<_>>()
                        .into_iter()
                })
                .collect()
        };

        if to_save.is_empty() {
            return Ok(());
        }

        let pwd = env::current_dir().unwrap();
        let Some(file_save_as) = path_choose::save_as(self.upcast_ref(), &pwd).await else {
            return Ok(());
        };

        if file_save_as.exists() {
            if file_save_as.is_file() {
                if !user_interaction::confirm(
                    self.upcast_ref(),
                    &format!("Do you want to overwrite?\n{}", file_save_as.display()),
                )
                .await
                {
                    return Ok(());
                }
            } else {
                self.show_error(format!("You can't overwrite {}", file_save_as.display()))
                    .await;
                return Ok(());
            }
        }

        save_file(&file_save_as, &to_save)?;
        Ok(())
    }

    fn get_selected_fs_path(&self) -> Option<PathBuf> {
        let (_position, item) = self.imp().view.single_selected()?;
        Some(item.path())
    }

    async fn do_rename(&self) -> Result<(), Box<dyn Error>> {
        let private = self.imp();
        let Some((_position, item)) = private.view.single_selected() else {
            return Ok(());
        };

        let old_path = item.path();
        let old_name = old_path
            .file_name()
            .ok_or("Cannot get base name of the file")?
            .to_str()
            .ok_or("Cannot convert file name to string")?
            .to_owned();

        let Some(new_name) =
            user_interaction::prompt(self.upcast_ref(), "Rename file", "Name:", &old_name)
                .await
                .filter(|name| *name != old_name)
        else {
            return Ok(());
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

        item.init_path(&new_path);

        Ok(())
    }

    async fn confirm_deletion(&self, count: usize) -> bool {
        if self.imp().confirm_delete.get() {
            let question = if count == 1 {
                "Are you sure you want to delete this file?".into()
            } else {
                format!("Are you sure you want to delete these {} files?", count)
            };
            let (confirm, ask_again) =
                user_interaction::confirm_delete(self.upcast_ref(), &question).await;
            self.imp().confirm_delete.set(ask_again);
            confirm
        } else {
            true
        }
    }

    async fn show_error(&self, message: impl ToString) {
        user_interaction::notify_error(self.upcast_ref(), &message.to_string()).await;
    }

    async fn find(&self) {
        let private = self.imp();

        let search_dirs = private.options.get_directories();
        if search_dirs.is_empty() {
            self.show_error("No search paths specified").await;
            return;
        }
        let excluded = private.options.get_excluded();

        let min_size: u64 = private.options.get_min_size();
        let recurse = private.options.get_recurse();

        private.duplicates.clear();

        let progress = user_interaction::ProgressDialog::new(self.upcast_ref(), "Searching...");
        *private.progress.borrow_mut() = Some(progress);

        let (sender, receiver) = async_channel::bounded(1);
        thread::spawn(move || {
            let duplicates = find_duplicate_groups(&search_dirs, &excluded, min_size, recurse);
            let _ = sender.send_blocking(duplicates.map_err(|err| err.to_string()));
        });
        let msg = receiver.recv().await.map_err(|e| e.to_string()).flatten();

        self.imp().on_find_finished(msg).await;
    }

    async fn open(&self) {
        if let Some(path) = self.get_selected_fs_path() {
            if let Err(error) = xdg_open(&path) {
                self.show_error(error).await;
            }
        }
    }

    async fn open_directory(&self) {
        if let Some(dir) = self
            .get_selected_fs_path()
            .and_then(|path| path.parent().map(|p| p.to_path_buf()))
        {
            if let Err(error) = xdg_open(&dir) {
                self.show_error(error).await;
            }
        }
    }

    fn copy(&self) {
        if let Some(path) = self.get_selected_fs_path() {
            self.clipboard().set_text(path.to_string_lossy().as_ref());
        }
    }

    async fn rename(&self) {
        if let Err(error) = self.do_rename().await {
            self.show_error(error).await;
        }
    }

    /// select all other duplicates from selected item folder
    fn select_from_same_directory(&self) {
        let private = self.imp();
        if let Some(path) = self.get_selected_fs_path()
            && let Some(dir) = path.parent()
        {
            let selection = private.view.selection();
            let mut position = 0;
            for group in private.duplicates.group_iter() {
                for file in group.iter() {
                    if file.path().parent() == Some(dir) {
                        selection.select_item(position, false);
                    }
                    position += 1;
                }
            }
        }
    }

    async fn select_wildcard(&self, select: bool) {
        let private = self.imp();
        if private.duplicates.is_empty() {
            return;
        }

        let title = if select {
            "Select by wildcard"
        } else {
            "Unselect by wildcard"
        };
        let Some(wildcard) = user_interaction::prompt(self.upcast_ref(), title, "Wildcard:", "*")
            .await
            .filter(|answer| !answer.is_empty())
        else {
            return;
        };

        let pattern = match glob::Pattern::new(&wildcard) {
            Ok(pattern) => pattern,
            Err(error) => {
                self.show_error(&error.to_string()).await;
                return;
            }
        };

        let selection = private.view.selection();
        let mut position = 0;
        for group in private.duplicates.group_iter() {
            for file in group.iter() {
                let fs_path = file.path();
                if pattern.matches_path(&fs_path) {
                    if select {
                        selection.select_item(position, false);
                    } else {
                        selection.unselect_item(position);
                    }
                }
                position += 1;
            }
        }
    }

    fn select_all_but(&self, which: String) {
        let skip = match which.as_str() {
            "first" => ItemWithinGroup::First,
            "newest" => ItemWithinGroup::Newest,
            "oldest" => ItemWithinGroup::Oldest,
            _ => return,
        };

        let private = self.imp();
        let selection = private.view.selection();
        let mut position = 0;
        for group in private.duplicates.group_iter() {
            let index_to_skip = match skip {
                ItemWithinGroup::First => Some(0),
                ItemWithinGroup::Newest => group.newest_position(),
                ItemWithinGroup::Oldest => group.oldest_position(),
            };
            for (index, _file) in group.iter().enumerate() {
                if Some(index) != index_to_skip {
                    selection.select_item(position, false);
                }
                position += 1;
            }
        }
    }

    fn select_toggle(&self) {
        let private = self.imp();

        let selection = private.view.selection();
        let mut position = 0;
        for group in private.duplicates.group_iter() {
            for _file in group.iter() {
                if selection.is_selected(position) {
                    selection.unselect_item(position);
                } else {
                    selection.select_item(position, false);
                }
                position += 1;
            }
        }
    }

    fn unselect_all(&self) {
        self.imp().view.selection().unselect_all();
    }

    async fn delete(&self) {
        let selected = self.imp().view.selected();

        let count = selected.len();
        if count == 0 {
            self.show_error("No file is selected").await;
            return;
        }
        if !self.confirm_deletion(count).await {
            return;
        }

        let mut deleted: HashSet<Duplicate> = HashSet::new();
        let mut errors = Vec::new();
        for (_position, item) in selected {
            let path = item.path();
            match fs::remove_file(&path) {
                Ok(_) => {
                    deleted.insert(item);
                }
                Err(error) => {
                    errors.push(format!(
                        "File {} cannot be removed. {}",
                        path.display(),
                        error
                    ));
                }
            }
        }

        self.imp().duplicates.remove_all(&deleted);

        if errors.is_empty() {
            user_interaction::notify_info(
                self.upcast_ref(),
                &format!("{} items deleted", deleted.len()),
            )
            .await;
        } else {
            let mut error_message = String::from("Following errors happened:\n");
            for error in errors {
                error_message.push('\n');
                error_message.push_str(&error.to_string());
            }
            user_interaction::notify_detailed(
                self.upcast_ref(),
                &format!("{} items deleted", deleted.len()),
                &error_message,
            )
            .await;
        }
    }

    async fn save(&self) {
        if let Err(error) = self.do_save().await {
            self.show_error(error).await;
        }
    }
}

fn save_file(destination_path: &Path, paths: &[PathBuf]) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(destination_path)?;

    for path in paths {
        file.write_all(path.to_str().unwrap().as_bytes())?;
        file.write_all(b"\n")?;
    }
    Ok(())
}
