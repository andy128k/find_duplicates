use crate::main_window::MainWindow;
use gtk::{gdk, gio, prelude::*};

pub fn create_application() -> gtk::Application {
    let app = gtk::Application::builder()
        .application_id("dev.andy128k.FindDuplicates")
        .flags(gio::ApplicationFlags::FLAGS_NONE)
        .build();

    app.connect_startup(|_app| {
        if let Some(display) = gdk::Display::default() {
            let style = include_str!("style.css");

            let provider = gtk::CssProvider::new();
            provider.load_from_string(style);

            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });

    app.connect_activate(|app| {
        let app_window = MainWindow::new(app);
        if let Ok(directory) = std::env::current_dir() {
            app_window.add_directory(&directory);
        }
        app_window.present();
    });

    app
}
