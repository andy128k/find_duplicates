#![macro_use]

pub struct ActionName(pub &'static str, pub usize);

macro_rules! action_name {
    ($scope:ident, $name:ident) => {
        crate::action_name::ActionName(
            concat!(stringify!($scope), ".", stringify!($name)),
            stringify!($scope).len() + 1,
        )
    };
}

impl ActionName {
    pub const fn full(&self) -> &'static str {
        self.0
    }

    pub fn local(&self) -> &'static str {
        &self.0[self.1..]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const SELECT_WILDCARD: ActionName = action_name!(win, select_wildcard);

    #[test]
    fn test_action_name() {
        assert_eq!(SELECT_WILDCARD.full(), "win.select_wildcard");
        assert_eq!(SELECT_WILDCARD.local(), "select_wildcard");
    }
}
