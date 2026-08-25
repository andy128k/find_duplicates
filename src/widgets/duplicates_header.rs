use gtk::{glib, prelude::*, subclass::prelude::*};

mod imp {
    use super::*;
    use crate::duplicate::Duplicate;
    use std::cell::{Cell, RefCell};

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::DuplicatesHeader)]
    pub struct DuplicatesHeader {
        label: gtk::Label,
        #[property(get, set = Self::set_group_size)]
        group_size: Cell<u32>,
        #[property(get, set = Self::set_probe)]
        probe: RefCell<Option<Duplicate>>,

        pub(super) bindings: RefCell<Vec<glib::Binding>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DuplicatesHeader {
        const NAME: &'static str = "FDDuplicatesHeader";
        type Type = super::DuplicatesHeader;
        type ParentType = gtk::Widget;
    }

    #[glib::derived_properties]
    impl ObjectImpl for DuplicatesHeader {
        fn constructed(&self) {
            self.parent_constructed();

            let this = self.obj();
            this.set_layout_manager(Some(gtk::BinLayout::new()));

            self.label.set_xalign(0.0);
            self.label.add_css_class("duplicates-header");
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

    impl WidgetImpl for DuplicatesHeader {}

    impl DuplicatesHeader {
        fn set_group_size(&self, group_size: u32) {
            self.group_size.set(group_size);
            self.update_label();
        }

        fn set_probe(&self, probe: Option<Duplicate>) {
            self.probe.replace(probe);
            self.update_label();
        }

        fn update_label(&self) {
            let group_size = self.group_size.get();
            let file_size = self
                .probe
                .borrow()
                .as_ref()
                .map(|d| d.size())
                .unwrap_or_default();
            self.label.set_label(&format!(
                "{} files x {} ({} wasted)",
                group_size,
                glib::format_size(file_size),
                glib::format_size((group_size - 1) as u64 * file_size)
            ));
        }
    }
}

glib::wrapper! {
    pub struct DuplicatesHeader(ObjectSubclass<imp::DuplicatesHeader>)
        @extends gtk::Widget,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl Default for DuplicatesHeader {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl DuplicatesHeader {
    pub fn bind(&self, header: &gtk::ListHeader) {
        self.imp().bindings.borrow_mut().push(
            header
                .bind_property("n-items", self, "group-size")
                .sync_create()
                .build(),
        );
        self.imp().bindings.borrow_mut().push(
            header
                .bind_property("item", self, "probe")
                .sync_create()
                .build(),
        );
    }

    pub fn unbind(&self, header: &gtk::ListHeader) {
        for binding in self
            .imp()
            .bindings
            .borrow_mut()
            .extract_if(.., |b| b.source().and_downcast_ref() == Some(header))
        {
            binding.unbind();
        }
    }
}
