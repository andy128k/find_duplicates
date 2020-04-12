use glib::clone::{Downgrade, Upgrade};
use glib::object::WeakRef;
use glib::ObjectType;
use std::marker::PhantomData;

pub struct NewTypeWeakRef<NT, T: ObjectType> {
    inner: WeakRef<T>,
    _phantom_newtype: PhantomData<NT>,
}

impl<NT, T: ObjectType> NewTypeWeakRef<NT, T> {
    pub fn from_inner(inner: WeakRef<T>) -> Self {
        Self {
            inner,
            _phantom_newtype: PhantomData::<NT>,
        }
    }
}

impl<NT: From<T>, T: ObjectType + Downgrade> Upgrade for NewTypeWeakRef<NT, T> {
    type Strong = NT;

    fn upgrade(&self) -> Option<Self::Strong> {
        Upgrade::upgrade(&self.inner).map(|upgraded_inner| NT::from(upgraded_inner))
    }
}
