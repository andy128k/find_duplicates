mod application;
mod duplicates_list;
mod exclusion;
mod find_duplicates;
mod main_window;
mod options;
mod path_choose;
mod string_list;
mod user_interaction;
mod utils;
mod widgets;

use gtk::prelude::*;
use std::process::ExitCode;

fn main() -> ExitCode {
    application::create_application().run().into()
}
