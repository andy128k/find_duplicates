use crate::action_name::ActionName;

pub trait MenuBuilderExt {
    fn item(self, label: &str, action: &ActionName) -> Self;
    fn submenu(self, label: &str, submenu: gio::Menu) -> Self;
}

impl MenuBuilderExt for gio::Menu {
    fn item(self, label: &str, action: &ActionName) -> Self {
        self.append_item(&gio::MenuItem::new(Some(label), Some(action.full())));
        self
    }

    fn submenu(self, label: &str, submenu: gio::Menu) -> Self {
        self.append_submenu(Some(label), &submenu);
        self
    }
}
