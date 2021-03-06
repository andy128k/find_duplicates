use chrono::prelude::*;
use glib::clone;
use gtk::prelude::*;
use std::iter::Peekable;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Clone)]
pub struct DuplicatesStore(gtk::ListStore);

enum StoreColumn {
    IsGroup = 0,
    Name = 1,
    Directory = 2,
    Time = 3,
    Size = 4,

    Path = 5,
    Modified = 6,
    Background = 7,
}

impl DuplicatesStore {
    pub fn new() -> Self {
        Self(gtk::ListStore::new(&[
            glib::Type::Bool,   // IsGroup
            glib::Type::String, // Name
            glib::Type::String, // Directory
            glib::Type::String, // Time
            glib::Type::String, // Size
            glib::Type::String, // Path
            glib::Type::String, // Modified
            glib::Type::String, // Background
        ]))
    }

    pub fn is_empty(&self) -> bool {
        self.0.get_iter_first().is_none()
    }

    pub fn append_group(&self, group_size: usize, file_size: u64) {
        let iter = self.0.append();
        self.0.set_value(
            &iter,
            StoreColumn::IsGroup as u32,
            &glib::Value::from(&true),
        );
        self.0.set_value(
            &iter,
            StoreColumn::Name as u32,
            &glib::Value::from(&format!("{} x {}", group_size, file_size)),
        );
        self.0.set_value(
            &iter,
            StoreColumn::Directory as u32,
            &glib::Value::from(&format!("{} wasted", (group_size - 1) as u64 * file_size)),
        );
        self.0
            .set_value(&iter, StoreColumn::Time as u32, &glib::Value::from(""));
        self.0.set_value(
            &iter,
            StoreColumn::Size as u32,
            &glib::Value::from(&format!("{}", group_size as u64 * file_size)),
        );
        self.0.set_value(
            &iter,
            StoreColumn::Background as u32,
            &glib::Value::from("#EEEEEE"),
        );
    }

    pub fn append_file(&self, path1: &Path, modified: SystemTime, file_size: u64) {
        let iter = self.0.append();
        self.0.set_value(
            &iter,
            StoreColumn::IsGroup as u32,
            &glib::Value::from(&false),
        );
        self.set_path(&iter, path1);
        let date: DateTime<Local> = modified.into();
        let time_str = if Local::now().signed_duration_since(date) > chrono::Duration::days(182) {
            date.format("%b %e %Y")
        } else {
            date.format("%b %e")
        }
        .to_string();
        self.0.set_value(
            &iter,
            StoreColumn::Time as u32,
            &glib::Value::from(&time_str),
        );
        self.0.set_value(
            &iter,
            StoreColumn::Size as u32,
            &glib::Value::from(&format!("{}", file_size)),
        );
        self.0.set_value(
            &iter,
            StoreColumn::Modified as u32,
            &glib::Value::from(&date.to_rfc3339()),
        );
    }

    pub fn set_path(&self, iter: &gtk::TreeIter, path: &Path) {
        self.0.set_value(
            &iter,
            StoreColumn::Name as u32,
            &glib::Value::from(path.file_name().unwrap().to_str().unwrap()),
        );
        self.0.set_value(
            &iter,
            StoreColumn::Directory as u32,
            &glib::Value::from(path.parent().unwrap().to_str().unwrap()),
        );
        self.0.set_value(
            &iter,
            StoreColumn::Path as u32,
            &glib::Value::from(path.to_str().unwrap()),
        );
    }

    pub fn get_fs_path(&self, iter: &gtk::TreeIter) -> Option<PathBuf> {
        let path: String = self
            .0
            .get_value(iter, StoreColumn::Path as i32)
            .get()
            .ok()??;
        Some(Path::new(&path).to_path_buf())
    }

    pub fn is_group(&self, iter: &gtk::TreeIter) -> bool {
        self.0
            .get_value(iter, StoreColumn::IsGroup as i32)
            .get()
            .unwrap()
            .unwrap()
    }

    pub fn modified(&self, iter: &gtk::TreeIter) -> DateTime<Local> {
        let s: String = self
            .0
            .get_value(iter, StoreColumn::Modified as i32)
            .get()
            .unwrap()
            .unwrap();
        DateTime::parse_from_rfc3339(&s)
            .unwrap()
            .with_timezone(&Local)
    }

    pub fn to_model(&self) -> gtk::ListStore {
        self.0.clone()
    }

    pub fn clear(&self) {
        self.0.clear();
    }

    pub fn iter(&self) -> DuplicatesStoreIter {
        let iter = self.0.get_iter_first();
        DuplicatesStoreIter {
            store: self.clone(),
            iter,
        }
    }

    pub fn group_iter(&self) -> DuplicatesStoreGroupIter {
        DuplicatesStoreGroupIter {
            store: self.clone(),
            iter: self.iter().peekable(),
        }
    }

    pub fn get_ref(&self, iter: &gtk::TreeIter) -> Option<gtk::TreeRowReference> {
        let path = self.0.get_path(iter)?;
        gtk::TreeRowReference::new(&self.0, &path)
    }

    fn remove_iters(&self, iters: &[gtk::TreeIter]) {
        let to_remove: Vec<gtk::TreeRowReference> =
            iters.iter().filter_map(|iter| self.get_ref(iter)).collect();

        for row_ref in to_remove {
            if let Some(path) = row_ref.get_path() {
                if let Some(iter) = self.0.get_iter(&path) {
                    self.0.remove(&iter);
                }
            }
        }
    }

    pub fn remove_all(&self, iters: &[gtk::TreeIter]) {
        self.remove_iters(iters);

        let mut to_remove: Vec<gtk::TreeIter> = Vec::new();
        for (group, files) in self.group_iter() {
            if files.len() <= 1 {
                to_remove.push(group);
                to_remove.extend(files);
            }
        }

        self.remove_iters(&to_remove)
    }
}

pub struct DuplicatesStoreIter {
    store: DuplicatesStore,
    iter: Option<gtk::TreeIter>,
}

impl Iterator for DuplicatesStoreIter {
    type Item = gtk::TreeIter;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.iter.as_ref()?.clone();
        let next = current.clone();
        if self.store.0.iter_next(&next) {
            self.iter = Some(next);
        } else {
            self.iter = None;
        }
        Some(current)
    }
}

pub struct DuplicatesStoreGroupIter {
    store: DuplicatesStore,
    iter: Peekable<DuplicatesStoreIter>,
}

impl DuplicatesStoreGroupIter {
    fn take_group(&mut self) -> Option<gtk::TreeIter> {
        loop {
            let iter = self.iter.next()?;
            if self.store.is_group(&iter) {
                return Some(iter);
            }
        }
    }

    fn take_files(&mut self) -> Vec<gtk::TreeIter> {
        let mut files = Vec::new();
        loop {
            if let Some(iter) = self.iter.peek() {
                if self.store.is_group(iter) {
                    break;
                } else {
                    files.push(self.iter.next().unwrap());
                }
            } else {
                break;
            }
        }
        files
    }
}

impl Iterator for DuplicatesStoreGroupIter {
    type Item = (gtk::TreeIter, Vec<gtk::TreeIter>);

    fn next(&mut self) -> Option<Self::Item> {
        let group = self.take_group()?;
        let files = self.take_files();
        Some((group, files))
    }
}

#[derive(Clone)]
pub struct DuplicatesList {
    scrolled_window: gtk::ScrolledWindow,
    tree_view: gtk::TreeView,
}

impl DuplicatesList {
    pub fn new(model: &DuplicatesStore) -> Self {
        let tree_view = gtk::TreeViewBuilder::new()
            .can_focus(true)
            .expand(true)
            .headers_visible(true)
            .model(&model.to_model())
            .build();

        {
            let column1 = gtk::TreeViewColumn::new();
            column1.set_sizing(gtk::TreeViewColumnSizing::Autosize);
            column1.set_expand(true);
            column1.set_title("Name");

            let text = gtk::CellRendererText::new();
            column1.pack_start(&text, true);
            column1.add_attribute(&text, "text", StoreColumn::Name as i32);
            column1.add_attribute(&text, "background-set", StoreColumn::IsGroup as i32);
            column1.add_attribute(&text, "background", StoreColumn::Background as i32);

            tree_view.append_column(&column1);
        }

        {
            let column1 = gtk::TreeViewColumn::new();
            column1.set_sizing(gtk::TreeViewColumnSizing::Autosize);
            column1.set_expand(true);
            column1.set_title("Directory");

            let text = gtk::CellRendererText::new();
            column1.pack_start(&text, true);
            column1.add_attribute(&text, "text", StoreColumn::Directory as i32);
            column1.add_attribute(&text, "background-set", StoreColumn::IsGroup as i32);
            column1.add_attribute(&text, "background", StoreColumn::Background as i32);

            tree_view.append_column(&column1);
        }

        {
            let column1 = gtk::TreeViewColumn::new();
            column1.set_sizing(gtk::TreeViewColumnSizing::Autosize);
            column1.set_expand(true);
            column1.set_title("Date");

            let text = gtk::CellRendererText::new();
            column1.pack_start(&text, true);
            column1.add_attribute(&text, "text", StoreColumn::Time as i32);
            column1.add_attribute(&text, "background-set", StoreColumn::IsGroup as i32);
            column1.add_attribute(&text, "background", StoreColumn::Background as i32);

            tree_view.append_column(&column1);
        }

        let selection = tree_view.get_selection();
        selection.set_mode(gtk::SelectionMode::Multiple);
        selection.set_select_function(Some(Box::new(
            |_selection, model: &gtk::TreeModel, path, _selected| {
                let iter = model.get_iter(path).unwrap();
                let is_group: bool = model
                    .get_value(&iter, StoreColumn::IsGroup as i32)
                    .get()
                    .unwrap()
                    .unwrap();
                !is_group
            },
        )));

        let scrolled_window = gtk::ScrolledWindowBuilder::new()
            .can_focus(true)
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .shadow_type(gtk::ShadowType::None)
            .window_placement(gtk::CornerType::TopLeft)
            .build();

        scrolled_window.add(&tree_view);

        Self {
            tree_view,
            scrolled_window,
        }
    }

    pub fn set_popup(&self, popup_model: &gio::MenuModel) {
        let popup = gtk::Menu::from_model(popup_model);
        popup.set_property_attach_widget(Some(&self.tree_view));

        self.tree_view.connect_button_press_event(
            clone!(@weak popup => @default-return Inhibit(false), move |view, event| {
                const GDK_BUTTON_SECONDARY: u32 = 3;
                let button = event.get_button();
                if button == GDK_BUTTON_SECONDARY {
                    view.grab_focus();

                    let (x, y) = event.get_position();
                    if let Some((Some(path), _, _, _)) = view.get_path_at_pos(x as i32, y as i32) {
                        view.set_cursor(&path, None::<&gtk::TreeViewColumn>, false);
                    }

                    popup.popup_at_pointer(Some(event));

                    Inhibit(true)
                } else {
                    Inhibit(false)
                }
            }),
        );

        self.tree_view.connect_popup_menu(
            clone!(@weak popup => @default-return false, move |view| {
                view.grab_focus();
                popup.popup_at_widget(view, gdk::Gravity::Center, gdk::Gravity::Center, None);
                true
            }),
        );
    }

    pub fn get_widget(&self) -> gtk::Widget {
        self.scrolled_window.clone().upcast()
    }

    pub fn get_selection(&self) -> gtk::TreeSelection {
        self.tree_view.get_selection()
    }

    pub fn get_selected_iters(&self) -> Vec<gtk::TreeIter> {
        let (selected, model) = self.tree_view.get_selection().get_selected_rows();
        selected
            .into_iter()
            .filter_map(|tree_path| model.get_iter(&tree_path))
            .collect()
    }

    pub fn get_selected_iter(&self) -> Option<gtk::TreeIter> {
        let mut selected = self.get_selected_iters();
        if selected.len() == 1 {
            selected.pop()
        } else {
            None
        }
    }
}
