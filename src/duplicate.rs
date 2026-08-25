use gtk::glib::{self, prelude::*, subclass::prelude::*};
use std::{
    error::Error,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

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
        modified_string: RefCell<Option<String>>,
        #[property(get, set)]
        size: Cell<u64>,
        #[property(get, set)]
        path: RefCell<PathBuf>,
        #[property(get, set, nullable)]
        modified: RefCell<Option<glib::DateTime>>,
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
    pub fn new(path: &Path, modified: SystemTime, file_size: u64) -> Self {
        let date = system_to_glib(modified)
            .inspect_err(|error| eprintln!("Bad date {error}"))
            .ok();
        let date_string = date
            .as_ref()
            .map(format_date)
            .transpose()
            .inspect_err(|error| eprintln!("Bad date format {error}"))
            .ok()
            .flatten();

        let this: Self = glib::Object::builder()
            .property("modified", date)
            .property("modified-string", date_string)
            .property("size", file_size)
            .build();

        this.init_path(path);
        this
    }

    pub fn init_path(&self, path: &Path) {
        self.set_name(path.file_name().unwrap().to_str().unwrap());
        self.set_directory(path.parent().unwrap().to_str().unwrap());
        self.set_path(path);
    }
}

fn system_to_glib(time: SystemTime) -> Result<glib::DateTime, Box<dyn Error>> {
    let timestamp = time.duration_since(UNIX_EPOCH)?.as_secs();
    let timestamp_signed: i64 = timestamp.try_into()?;
    let date = glib::DateTime::from_unix_utc(timestamp_signed)?;
    Ok(date)
}

fn format_date(date: &glib::DateTime) -> Result<String, Box<dyn Error>> {
    let now = glib::DateTime::now_local()?;
    let format = if now.year() != date.year() {
        "%b %e %Y"
    } else {
        "%b %e"
    };
    Ok(date.format(format)?.to_string())
}
