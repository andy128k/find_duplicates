use std::{path::Path, time::SystemTime};

use chrono::{DateTime, Local};
use gtk::glib::{self, prelude::*, subclass::prelude::*};

mod imp {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        path::PathBuf,
    };

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::Duplicate)]
    pub struct Duplicate {
        #[property(get, set)]
        name: RefCell<String>,
        #[property(get, set)]
        directory: RefCell<String>,
        #[property(get, set)]
        time: RefCell<String>,
        #[property(get, set)]
        size: Cell<u64>,
        #[property(get, set)]
        path: RefCell<PathBuf>,
        #[property(get, set)]
        modified: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Duplicate {
        const NAME: &'static str = "Duplicate";
        type Type = super::Duplicate;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Duplicate {}
}

glib::wrapper! {
    pub struct Duplicate(ObjectSubclass<imp::Duplicate>);
}

impl Duplicate {
    pub fn new(path1: &Path, modified: SystemTime, file_size: u64) -> Self {
        let date: DateTime<Local> = modified.into();

        let this: Self = glib::Object::builder()
            .property(
                "time",
                if Local::now().signed_duration_since(date) > chrono::Duration::days(182) {
                    date.format("%b %e %Y")
                } else {
                    date.format("%b %e")
                }
                .to_string(),
            )
            .property("modified", date.to_rfc3339())
            .property("size", file_size)
            .build();

        this.init_path(path1);
        this
    }

    pub fn init_path(&self, path: &Path) {
        self.set_name(path.file_name().unwrap().to_str().unwrap());
        self.set_directory(path.parent().unwrap().to_str().unwrap());
        self.set_path(path);
    }

    pub fn modified_date(&self) -> DateTime<Local> {
        DateTime::parse_from_rfc3339(&self.modified())
            .unwrap()
            .with_timezone(&Local)
    }
}
