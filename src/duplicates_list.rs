use crate::widgets::duplicates_header::DuplicatesHeader;
use crate::widgets::property_label::PropertyLabel;
use crate::{duplicate::Duplicate, utils::scrolled};
use gtk::{
    gdk::{self, ffi::GDK_BUTTON_SECONDARY},
    gio, glib,
    prelude::*,
    subclass::prelude::*,
};
use std::time::SystemTime;
use std::{collections::HashSet, path::Path};

pub struct Duplicates(gio::ListStore);

impl Duplicates {
    pub fn new() -> Self {
        Self(gio::ListStore::new::<Duplicate>())
    }

    pub fn iter(&self) -> impl Iterator<Item = Duplicate> {
        self.0.iter::<Duplicate>().flatten()
    }

    pub fn len(&self) -> u32 {
        self.0.n_items()
    }

    pub fn newest_position(&self) -> Option<usize> {
        self.iter()
            .enumerate()
            .max_by_key(|(_i, item)| item.modified())
            .map(|(i, _item)| i)
    }

    pub fn oldest_position(&self) -> Option<usize> {
        self.iter()
            .enumerate()
            .min_by_key(|(_i, item)| item.modified())
            .map(|(i, _item)| i)
    }

    pub fn append_file(&self, path: &Path, modified: SystemTime, file_size: u64) {
        let item = Duplicate::new(path, modified, file_size);
        self.0.append(&item);
    }

    fn remove_items(&self, items: &HashSet<Duplicate>) {
        let positions: Vec<_> = self
            .iter()
            .enumerate()
            .filter_map(|(pos, item)| items.contains(&item).then_some(pos))
            .collect();
        for p in positions.iter().rev() {
            self.0.remove(*p as u32);
        }
    }
}

#[derive(Clone)]
pub struct DuplicatesStore(gio::ListStore);

impl Default for DuplicatesStore {
    fn default() -> Self {
        Self(gio::ListStore::new::<gio::ListStore>())
    }
}

impl DuplicatesStore {
    pub fn is_empty(&self) -> bool {
        self.0.n_items() == 0
    }

    pub fn append_group(&self, group: Duplicates) {
        self.0.append(&group.0);
    }

    pub fn clear(&self) {
        self.0.remove_all();
    }

    pub fn group_iter(&self) -> impl Iterator<Item = Duplicates> {
        self.0.iter::<gio::ListStore>().flatten().map(Duplicates)
    }

    fn remove_items(&self, items: &HashSet<Duplicate>) {
        for group in self.group_iter() {
            group.remove_items(items);
        }
    }

    pub fn remove_all(&self, items: &HashSet<Duplicate>) {
        self.remove_items(items);
        let positions: Vec<_> = self
            .group_iter()
            .enumerate()
            .filter_map(|(pos, group)| (group.len() <= 1).then_some(pos))
            .collect();
        for p in positions.iter().rev() {
            self.0.remove(*p as u32);
        }
    }
}

mod imp {
    use super::*;
    use std::cell::RefCell;

    pub struct DuplicatesList {
        pub view: gtk::ColumnView,
        pub selection: gtk::MultiSelection,
        pub model: RefCell<DuplicatesStore>,
        pub popup: gtk::PopoverMenu,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DuplicatesList {
        const NAME: &'static str = "FDDuplicatesList";
        type Type = super::DuplicatesList;
        type ParentType = gtk::Widget;

        fn new() -> Self {
            Self {
                view: gtk::ColumnView::builder().can_focus(true).build(),
                selection: gtk::MultiSelection::new(None::<gio::ListModel>),
                model: Default::default(),
                popup: gtk::PopoverMenu::from_model(gio::MenuModel::NONE),
            }
        }
    }

    impl ObjectImpl for DuplicatesList {
        fn constructed(&self) {
            self.parent_constructed();

            let this = self.obj();
            this.set_layout_manager(Some(gtk::BinLayout::new()));

            self.view.set_model(Some(&self.selection));
            self.view.set_header_factory(Some(&header_factory()));

            fn column(title: &str, text_column: &'static str) -> gtk::ColumnViewColumn {
                gtk::ColumnViewColumn::builder()
                    .expand(true)
                    .title(title)
                    .factory(&label_factory(text_column))
                    .build()
            }

            self.view.append_column(&column("Name", "name"));
            self.view.append_column(&column("Directory", "directory"));
            self.view.append_column(&column("Date", "modified-string"));

            let scrolled_window = scrolled(&self.view, false);
            scrolled_window.set_parent(&*this);

            self.popup.set_parent(&*this);

            let click = gtk::GestureClick::builder()
                .button(GDK_BUTTON_SECONDARY as u32)
                .build();
            click.connect_pressed(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_gesture, _n, x, y| {
                    imp.view.grab_focus();
                    imp.popup
                        .set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 0, 0)));
                    imp.popup.popup();
                }
            ));
            self.view.add_controller(click);
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for DuplicatesList {}
}

glib::wrapper! {
    pub struct DuplicatesList(ObjectSubclass<imp::DuplicatesList>)
        @extends gtk::Widget,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl Default for DuplicatesList {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl DuplicatesList {
    pub fn set_model(&self, model: &DuplicatesStore) {
        self.imp().model.replace(model.clone());
        let model = gtk::FlattenListModel::new(Some(model.0.clone()));
        self.selection().set_model(Some(&model));
    }

    pub fn set_popup(&self, popup_model: &gio::MenuModel) {
        self.imp().popup.set_menu_model(Some(popup_model));
    }

    pub fn selection(&self) -> gtk::MultiSelection {
        self.imp().view.model().and_downcast().unwrap()
    }

    pub fn selected(&self) -> Vec<(u32, Duplicate)> {
        let selection = self.selection();
        let selected = selection.selection();
        (0..(selected.size() as u32))
            .map(move |i| selected.nth(i))
            .filter_map(|position| {
                Some((
                    position,
                    selection.item(position).and_downcast::<Duplicate>()?,
                ))
            })
            .collect()
    }

    pub fn single_selected(&self) -> Option<(u32, Duplicate)> {
        let selection = self.selection();
        let selected = selection.selection();
        if selected.size() == 1 {
            let position = selected.minimum();
            let item = selection.item(position).and_downcast::<Duplicate>()?;
            Some((position, item))
        } else {
            None
        }
    }
}

fn header_factory() -> gtk::ListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, obj| {
        if let Some(header) = obj.downcast_ref::<gtk::ListHeader>() {
            header.set_child(Some(&DuplicatesHeader::default()));
        }
    });
    factory.connect_bind(|_, obj| {
        if let Some(header) = obj.downcast_ref::<gtk::ListHeader>()
            && let Some(child) = header.child().and_downcast::<DuplicatesHeader>()
        {
            child.bind(header);
        }
    });
    factory.connect_unbind(|_, obj| {
        if let Some(header) = obj.downcast_ref::<gtk::ListHeader>()
            && let Some(child) = header.child().and_downcast::<DuplicatesHeader>()
        {
            child.unbind(header);
        }
    });
    factory.upcast()
}

fn label_factory(property: &'static str) -> gtk::ListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, obj| {
        if let Some(list_item) = obj.downcast_ref::<gtk::ListItem>() {
            list_item.set_child(Some(&PropertyLabel::new(property)));
        }
    });
    factory.connect_bind(|_, obj| {
        if let Some(list_item) = obj.downcast_ref::<gtk::ListItem>()
            && let Some(label) = list_item.child().and_downcast::<PropertyLabel>()
            && let Some(item) = list_item.item()
        {
            label.bind(&item);
        }
    });
    factory.connect_unbind(|_, obj| {
        if let Some(list_item) = obj.downcast_ref::<gtk::ListItem>()
            && let Some(label) = list_item.child().and_downcast::<PropertyLabel>()
            && let Some(item) = list_item.item()
        {
            label.unbind(&item);
        }
    });
    factory.upcast()
}
