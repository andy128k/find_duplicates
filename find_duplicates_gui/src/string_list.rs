use crate::phantom_data_weak::PhantomData;
use gtk::prelude::*;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Clone, supplemental_macros::GlibDowngrade)]
pub struct StringList<T>(gtk::ScrolledWindow, PhantomData<T>);

impl<T> StringList<T> {
    pub fn new() -> Self {
        let model = gtk::ListStore::new(&[glib::Type::String, glib::Type::String]);

        let view = gtk::TreeViewBuilder::new()
            .can_focus(true)
            .expand(true)
            .headers_visible(false)
            .model(&model)
            .build();
        view.get_selection().set_mode(gtk::SelectionMode::Multiple);

        let column = gtk::TreeViewColumn::new();
        column.set_sizing(gtk::TreeViewColumnSizing::Autosize);
        column.set_expand(true);

        let text = gtk::CellRendererText::new();
        column.pack_start(&text, true);
        column.add_attribute(&text, "text", 0);

        view.append_column(&column);

        let scrolled_window = gtk::ScrolledWindowBuilder::new()
            .can_focus(true)
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .shadow_type(gtk::ShadowType::In)
            .window_placement(gtk::CornerType::TopLeft)
            .build();

        scrolled_window.add(&view);

        Self(scrolled_window, PhantomData::new())
    }

    pub fn get_widget(&self) -> gtk::Widget {
        self.0.clone().upcast()
    }

    fn get_view(&self) -> gtk::TreeView {
        self.0.get_child().unwrap().downcast().unwrap()
    }

    fn get_model(&self) -> gtk::ListStore {
        self.get_view().get_model().unwrap().downcast().unwrap()
    }

    pub fn clear(&self) {
        self.get_model().clear()
    }

    pub fn remove_selection(&self) {
        let view = self.get_view();
        let model = self.get_model();
        remove_selection(&view, &model);
    }
}

impl<T: ToString + Serialize + DeserializeOwned> StringList<T> {
    pub fn append(&self, value: T) {
        let bytes = bincode::serialize(&value).expect("Bincode serializes value");
        let hex = hex::encode(&bytes);

        let model = self.get_model();
        let iter = model.append();
        model.set_value(&iter, 0, &glib::Value::from(&value.to_string()));
        model.set_value(&iter, 1, &glib::Value::from(&hex));
    }

    pub fn to_vec(&self) -> Vec<T> {
        let mut result: Vec<T> = Vec::new();
        self.get_model().foreach(|model, _path, iter| {
            let hex: String = model.get_value(iter, 1).get().unwrap().unwrap();
            let bytes = hex::decode(&hex).unwrap();
            let value = bincode::deserialize(&bytes).expect("Bincode deserializes value");
            result.push(value);
            false
        });
        result
    }
}

fn remove_selection(view: &gtk::TreeView, store: &gtk::ListStore) {
    let (selected, model) = view.get_selection().get_selected_rows();
    let row_refs: Vec<gtk::TreeRowReference> = selected
        .into_iter()
        .filter_map(|path| gtk::TreeRowReference::new(&model, &path))
        .collect();

    for row_ref in row_refs {
        if let Some(path) = row_ref.get_path() {
            if let Some(iter) = model.get_iter(&path) {
                store.remove(&iter);
            }
        }
    }
}
