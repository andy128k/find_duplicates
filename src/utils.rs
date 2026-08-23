use gtk::{glib, prelude::*};
use std::time::Duration;

pub fn scrolled(child: &impl IsA<gtk::Widget>, has_frame: bool) -> gtk::ScrolledWindow {
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
        if let Some((iter, position)) = gtk::BitsetIter::init_first(self) {
            indexes.push(position);
            for position in iter {
                indexes.push(position);
            }
        }
        indexes
    }
}

pub trait SenderExt<T> {
    fn toss(&self, message: T);
}

impl<T> SenderExt<T> for async_channel::Sender<T> {
    fn toss(&self, message: T) {
        if !self.is_closed()
            && let Err(error) = self.try_send(message)
        {
            eprintln!("Failed to send a message: {error}");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bitset_to_vec() {
        gtk::init().unwrap();

        let bitset = gtk::Bitset::new_empty();
        assert_eq!(bitset.to_vec(), vec![]);

        let bitset = gtk::Bitset::new_empty();
        bitset.add(42);
        bitset.add(47);
        bitset.add(7);

        assert_eq!(bitset.to_vec(), vec![7, 42, 47]);
    }
}
