use gtk::prelude::*;

#[derive(Clone)]
pub struct Errors {
    scrolled_window: gtk::ScrolledWindow,
    text_view: gtk::TextView,
}

impl Errors {
    pub fn new() -> Self {
        let text_view = gtk::TextViewBuilder::new()
            .can_focus(true)
            .editable(false)
            .overwrite(false)
            .accepts_tab(true)
            .justification(gtk::Justification::Left)
            .wrap_mode(gtk::WrapMode::None)
            .cursor_visible(true)
            .build();

        let scrolled_window = gtk::ScrolledWindowBuilder::new()
            .can_focus(true)
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .shadow_type(gtk::ShadowType::In)
            .window_placement(gtk::CornerType::TopLeft)
            .build();

        scrolled_window.add(&text_view);

        Self {
            text_view,
            scrolled_window,
        }
    }

    pub fn get_widget(&self) -> gtk::Widget {
        self.scrolled_window.clone().upcast()
    }

    pub fn append(&self, message: &str) {
        let buffer = self.text_view.get_buffer().unwrap();
        let mut iter = buffer.get_end_iter();
        buffer.insert(&mut iter, "\n\n");
        buffer.insert(&mut iter, message);
    }

    pub fn clear(&self) {
        self.text_view.get_buffer().unwrap().set_text("");
    }
}
