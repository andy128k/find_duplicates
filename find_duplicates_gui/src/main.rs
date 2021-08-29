mod application;
mod duplicates_list;
mod exclusion;
mod find_duplicates;
mod main_window;
mod options;
mod path_choose;
mod phantom_data_weak;
mod string_list;
mod user_interaction;
mod utils;
mod widgets;

use gio::prelude::*;

fn main() {
    let exit_status = application::create_application().run();
    std::process::exit(exit_status);
}
