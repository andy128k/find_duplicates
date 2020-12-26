#[derive(Clone)]
pub struct PhantomData<T>(std::marker::PhantomData<T>);
pub struct PhantomDataWeak<T>(std::marker::PhantomData<T>);

impl<T> PhantomData<T> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T> glib::clone::Downgrade for PhantomData<T> {
    type Weak = PhantomDataWeak<T>;

    fn downgrade(&self) -> Self::Weak {
        PhantomDataWeak(std::marker::PhantomData)
    }
}

impl<T> glib::clone::Upgrade for PhantomDataWeak<T> {
    type Strong = PhantomData<T>;

    fn upgrade(&self) -> Option<Self::Strong> {
        Some(PhantomData(std::marker::PhantomData))
    }
}
