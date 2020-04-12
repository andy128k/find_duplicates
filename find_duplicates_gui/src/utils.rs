use gtk::prelude::*;

pub fn horizontal_expander() -> gtk::Widget {
    gtk::DrawingAreaBuilder::new()
        .hexpand(true)
        .height_request(0)
        .build()
        .upcast()
}
