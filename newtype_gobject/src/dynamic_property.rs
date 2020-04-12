use crate::object_data::*;
use glib::{IsA, Object};
use std::marker::PhantomData;

pub struct DynamicProperty<T> {
    name: &'static str,
    _phantom_type: PhantomData<T>,
}

impl<T> DynamicProperty<T> {
    pub const fn new(name: &'static str) -> Self {
        DynamicProperty {
            name,
            _phantom_type: PhantomData::<T>,
        }
    }

    #[inline]
    pub fn set<O: IsA<Object>>(&self, obj: &O, value: T) {
        object_set_data(obj.as_ref(), self.name, Box::new(value))
    }

    #[inline]
    pub fn get<'obj, O: IsA<Object>>(&self, obj: &'obj O) -> Option<&'obj T> {
        object_get_data(obj.as_ref(), self.name)
    }
}
