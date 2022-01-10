use crate::gtk_prelude::*;
use std::time::Duration;

pub fn horizontal_expander() -> gtk::Widget {
    gtk::DrawingArea::builder()
        .hexpand(true)
        .height_request(0)
        .build()
        .upcast()
}

pub async fn pending() {
    glib::timeout_future(Duration::from_millis(1)).await;
}
