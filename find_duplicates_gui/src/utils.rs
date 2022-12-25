use crate::gtk_prelude::*;
use std::time::Duration;

pub fn scrolled(child: &impl glib::IsA<gtk::Widget>, has_frame: bool) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .can_focus(true)
        .hexpand(true)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .has_frame(has_frame)
        .window_placement(gtk::CornerType::TopLeft)
        .child(child)
        .build()
}

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

pub trait BitsetExt {
    fn to_vec(&self) -> Vec<u32>;
}

impl BitsetExt for gtk::Bitset {
    fn to_vec(&self) -> Vec<u32> {
        let mut indexes = Vec::new();
        if let Some((mut iter, position)) = gtk::BitsetIter::init_first(self) {
            indexes.push(position);
            while let Some(position) = iter.next() {
                indexes.push(position);
            }
        }
        indexes
    }
}
