use crate::duplicates_list;
use crate::exclusion::{Exclusion, DEFAULT_EXCLUDE_PATTERNS};
use crate::find_duplicates::{duplication_status, find_duplicate_groups, DuplicatesGroup};
use crate::gtk_prelude::*;
use crate::options;
use crate::path_choose;
use crate::user_interaction::{self, ProgressDialog};
use crate::utils::horizontal_expander;
use crate::widgets::go_button::go_button;
use crate::widgets::menu_builder::MenuBuilderExt;
use gtk::subclass::prelude::*;
use std::cell::{Cell, RefCell};
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
        .item("Select using wildcard", "win.select_wildcard(true)")
        .item("Unselect using wildcard", "win.select_wildcard(false)")
        .submenu(
            "Select within groups",
            gio::Menu::new()
                .item("Select all but first", r#"win.select_all_but("first")"#)
                .item("Select all but newest", r#"win.select_all_but("newest")"#)
                .item("Select all but oldest", r#"win.select_all_but("oldest")"#),
        )
        .item("Toggle selection", "win.select_toggle")
        .item("Unselect all", "win.unselect_all");

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
        .item("Open directory", "win.open_directory")
        .item("Copy", "win.copy")
        .item("Rename...", "win.rename")
        .item(
            "Select all in this directory",
            "win.select_from_same_directory",
        )
}

type FindResult = Result<Vec<DuplicatesGroup>, String>;

mod imp {
    use super::*;
    use gtk::glib::once_cell::sync::OnceCell;

    #[derive(Default)]
    pub struct MainWindow {
        pub confirm_delete: Cell<bool>,
        pub duplicates: duplicates_list::DuplicatesStore,
        pub options: options::Options,
        pub view: duplicates_list::DuplicatesList,
        pub find_sender: OnceCell<glib::Sender<FindResult>>,
        pub progress: RefCell<Option<ProgressDialog>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MainWindow {
        const NAME: &'static str = "MainWindow";
        type Type = super::MainWindow;
        type ParentType = gtk::ApplicationWindow;
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
                &results_layout(&self.view.get_widget(), &action_buttons),
            );

            window.set_child(Some(&paned));

            let (find_sender, find_receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
            self.find_sender.set(find_sender).unwrap();

            window.register_actions(&*window);
            find_receiver.attach(
                None,
                clone!(@weak self as imp => @default-return glib::Continue(false), move |msg| {
                    glib::MainContext::default().spawn_local(async move {
                        imp.on_find_finished(msg).await;
                    });
                    glib::Continue(true)
                }),
            );

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
        async fn on_find_finished(&self, msg: FindResult) {
            if let Some(progress) = self.progress.borrow_mut().take() {
                progress.close().await;
            }

            match msg {
                Ok(duplicates) => {
                    for group in &duplicates {
                        self.duplicates
                            .append_group(group.files.len(), group.size());
                        for fi in &group.files {
                            self.duplicates.append_file(&fi.path, fi.modified, fi.size);
                        }
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
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget, @implements gio::ActionMap;
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
        let selected = private.view.get_selected_iters();
        let to_save: Vec<PathBuf> = if !selected.is_empty() {
            selected
                .iter()
                .filter_map(|iter| private.duplicates.get_fs_path(iter))
                .collect()
        } else {
            private
                .duplicates
                .group_iter()
                .flat_map(|(_group, files)| files.into_iter())
                .filter_map(|iter| private.duplicates.get_fs_path(&iter))
                .collect()
        };

        if to_save.is_empty() {
            return Ok(());
        }

        let pwd = env::current_dir().unwrap();
        let Some(file_save_as) = path_choose::save_as(self.upcast_ref(), &pwd).await else { return Ok(()) };

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
        let private = self.imp();
        let iter = private.view.get_selected_iter()?;
        private.duplicates.get_fs_path(&iter)
    }

    fn get_path_and_name(&self, iter: &gtk::TreeIter) -> Result<(PathBuf, String), Box<dyn Error>> {
        let private = self.imp();
        let path = private
            .duplicates
            .get_fs_path(&iter)
            .ok_or("Cannot get path of the file")?;
        let name = path
            .file_name()
            .ok_or("Cannot get base name of the file")?
            .to_str()
            .ok_or("Cannot convert file name to string")?
            .to_owned();
        Ok((path, name))
    }

    async fn do_rename(&self) -> Result<(), Box<dyn Error>> {
        let private = self.imp();
        let Some(iter) = private.view.get_selected_iter() else { return Ok(()) };

        let (old_path, old_name) = self.get_path_and_name(&iter)?;

        let Some(new_name) = user_interaction::prompt(self.upcast_ref(), "Rename file", "Name:", &old_name)
            .await
            .filter(|name| *name != old_name)
            else { return Ok(()) };

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

        private.duplicates.set_path(&iter, &new_path);

        Ok(())
    }

    fn delete_file_by_tree_iter(&self, iter: &gtk::TreeIter) -> Result<(), Box<dyn Error>> {
        let private = self.imp();
        let fs_path = private
            .duplicates
            .get_fs_path(&iter)
            .ok_or("Cannot get path to file by iter.")?;
        fs::remove_file(&fs_path)
            .map_err(|e| format!("File {} cannot be removed. {}", fs_path.display(), e))?;
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
}

#[awesome_glib::actions]
impl MainWindow {
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

        let sender = private.find_sender.get().unwrap().clone();
        thread::spawn(move || {
            let duplicates = find_duplicate_groups(&search_dirs, &excluded, min_size, recurse);
            let _ = sender.send(duplicates.map_err(|err| err.to_string()));
        });
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
        if let Some(path) = self.get_selected_fs_path() {
            if let Some(dir) = path.parent() {
                for (_group, files) in private.duplicates.group_iter() {
                    for file in files {
                        if private
                            .duplicates
                            .get_fs_path(&file)
                            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                            == Some(dir.to_path_buf())
                        {
                            private.view.get_selection().select_iter(&file);
                        }
                    }
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
            else { return };

        let pattern = match glob::Pattern::new(&wildcard) {
            Ok(pattern) => pattern,
            Err(error) => {
                self.show_error(&error.to_string()).await;
                return;
            }
        };

        let selection = private.view.get_selection();
        for (_group, files) in private.duplicates.group_iter() {
            for file_iter in files {
                let fs_path = private.duplicates.get_fs_path(&file_iter).unwrap();
                if pattern.matches_path(&fs_path) {
                    if select {
                        selection.select_iter(&file_iter);
                    } else {
                        selection.unselect_iter(&file_iter);
                    }
                }
            }
        }
    }

    fn select_all_but(&self, which: String) {
        fn find_row_to_unselect<'i>(
            model: &duplicates_list::DuplicatesStore,
            files: &'i [gtk::TreeIter],
            which: &str,
        ) -> Option<&'i gtk::TreeIter> {
            match which {
                "first" => files.first(),
                "newest" => files.iter().max_by_key(|iter| model.modified(iter)),
                "oldest" => files.iter().min_by_key(|iter| model.modified(iter)),
                _ => None,
            }
        }

        let private = self.imp();
        let selection = private.view.get_selection();
        for (_group, files) in private.duplicates.group_iter() {
            for file in &files {
                selection.select_iter(file);
            }
            if let Some(unselect) = find_row_to_unselect(&private.duplicates, &files, &which) {
                selection.unselect_iter(unselect);
            }
        }
    }

    fn select_toggle(&self) {
        let private = self.imp();

        let selection = private.view.get_selection();
        for iter in private.duplicates.iter() {
            if !private.duplicates.is_group(&iter) {
                if selection.iter_is_selected(&iter) {
                    selection.unselect_iter(&iter);
                } else {
                    selection.select_iter(&iter);
                }
            }
        }
    }

    fn unselect_all(&self) {
        self.imp().view.get_selection().unselect_all();
    }

    async fn delete(&self) {
        let selected = self.imp().view.get_selected_iters();

        let count = selected.len();
        if count == 0 {
            self.show_error("No file is selected").await;
            return;
        }
        if !self.confirm_deletion(count).await {
            return;
        }

        let mut deleted: Vec<gtk::TreeIter> = Vec::new();
        let mut errors = Vec::new();
        for iter in selected {
            match self.delete_file_by_tree_iter(&iter) {
                Ok(_) => {
                    deleted.push(iter);
                }
                Err(error) => {
                    errors.push(error);
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
