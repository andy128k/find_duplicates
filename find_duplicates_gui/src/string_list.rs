use crate::gtk_prelude::*;
use crate::utils::BitsetExt;
use std::marker::PhantomData;

fn label_factory(
    display: impl Fn(&glib::Object) -> Option<String> + 'static,
) -> gtk::ListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, list_item| {
        list_item.set_child(Some(
            &gtk::Label::builder().halign(gtk::Align::Start).build(),
        ))
    });
    factory.connect_bind(move |_, list_item| {
        if let Some(label) = list_item.child().and_downcast_ref::<gtk::Label>() {
            let text = list_item.item().and_then(|obj| display(&obj));
            label.set_text(text.as_deref().unwrap_or_default());
        }
    });
    factory.connect_unbind(|_, list_item| {
        if let Some(label) = list_item.child().and_downcast_ref::<gtk::Label>() {
            label.set_text("");
        }
    });
    factory.connect_teardown(|_, list_item| list_item.set_child(gtk::Widget::NONE));
    factory.upcast()
}

#[derive(Clone, glib::Downgrade)]
pub struct StringList<T>(gtk::ListView, gio::ListStore, PhantomData<T>);

fn display_boxed<T: ToString + 'static>(obj: &glib::Object) -> Option<String> {
    obj.downcast_ref::<glib::BoxedAnyObject>()
        .map(|b| b.borrow::<T>().to_string())
}

impl<T: ToString + 'static> StringList<T> {
    pub fn new() -> Self {
        let model = gio::ListStore::new(glib::BoxedAnyObject::static_type());

        let selection_model = gtk::MultiSelection::new(Some(&model));

        let factory = label_factory(display_boxed::<T>);

        let view = gtk::ListView::builder()
            .can_focus(true)
            .hexpand(true)
            .vexpand(true)
            .model(&selection_model)
            .factory(&factory)
            .build();

        Self(view, model, PhantomData)
    }

    pub fn get_widget(&self) -> gtk::Widget {
        self.0.clone().upcast()
    }

    fn get_model(&self) -> &gio::ListStore {
        &self.1
    }

    pub fn clear(&self) {
        self.get_model().remove_all()
    }

    pub fn remove_selection(&self) {
        for position in self.0.model().unwrap().selection().to_vec().iter().rev() {
            self.get_model().remove(*position);
        }
    }
}

impl<T: ToString + Clone + 'static> StringList<T> {
    pub fn append(&self, value: T) {
        let item = glib::BoxedAnyObject::new(value);
        self.get_model().append(&item);
    }

    pub fn to_vec(&self) -> Vec<T> {
        self.get_model()
            .iter::<glib::BoxedAnyObject>()
            .unwrap()
            .map(|bx| (*bx.unwrap().borrow::<T>()).clone())
            .collect()
    }
}
