use crate::main_window::MainWindow;
use gio::prelude::*;

pub fn create_application() -> gtk::Application {
    let app = gtk::Application::new(
        Some("net.andy128k.FindDuplicates"),
        gio::ApplicationFlags::FLAGS_NONE,
    )
    .unwrap();

    // let app = gtk::ApplicationBuilder::new()
    //     .application_id("net.andy128k.FindDuplicates")
    //     .flags(gio::ApplicationFlags::FLAGS_NONE)
    //     .build();

    app.connect_activate(|app| {
        let app_window = MainWindow::new(app);
        if let Ok(directory) = std::env::current_dir() {
            app_window.add_directory(&directory);
        }
        app_window.show_all();
    });

    app
}
