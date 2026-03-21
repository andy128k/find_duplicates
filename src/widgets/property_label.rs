use gtk::{glib, prelude::*, subclass::prelude::*};

mod imp {
    use super::*;
    use std::cell::{OnceCell, RefCell};

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::PropertyLabel)]
    pub struct PropertyLabel {
        #[property(get)]
        label: gtk::Label,
        #[property(get, construct_only)]
        property_name: OnceCell<String>,

        pub(super) bindings: RefCell<Vec<glib::Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PropertyLabel {
        const NAME: &'static str = "FDPropertyLabel";
        type Type = super::PropertyLabel;
        type ParentType = gtk::Widget;
    }

    #[glib::derived_properties]
    impl ObjectImpl for PropertyLabel {
        fn constructed(&self) {
            self.parent_constructed();

            let this = self.obj();
            this.set_layout_manager(Some(gtk::BinLayout::new()));

            self.label.set_xalign(0.0);
            self.label.set_parent(&*this);
        }

        fn dispose(&self) {
            for b in self.bindings.take() {
                b.unbind();
            }

            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for PropertyLabel {}
}

glib::wrapper! {
    pub struct PropertyLabel(ObjectSubclass<imp::PropertyLabel>)
        @extends gtk::Widget,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl PropertyLabel {
    pub fn new(property_name: &str) -> Self {
        glib::Object::builder()
            .property("property-name", property_name)
            .build()
    }

    pub fn bind(&self, object: &glib::Object) {
        self.imp().bindings.borrow_mut().push(
            object
                .bind_property(&self.property_name(), &self.label(), "label")
                .sync_create()
                .build(),
        );
    }

    pub fn unbind(&self, object: &glib::Object) {
        for binding in self
            .imp()
            .bindings
            .borrow_mut()
            .extract_if(.., |b| b.source().and_downcast_ref() == Some(object))
        {
            binding.unbind();
        }
    }
}
