use gio::ListStore;
use glib::Object;
use gtk::{self, gio, glib, prelude::*};
use std::ops::{Deref, DerefMut};

pub mod imp {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct TypedListStore<T: IsA<Object>>(ListStore, std::marker::PhantomData<T>);

    impl<T: IsA<Object>> TypedListStore<T> {
        pub fn new() -> Self {
            Self(ListStore::new::<T>(), std::marker::PhantomData)
        }

        pub fn get(&self, index: u32) -> Option<T> {
            self.item(index).and_then(|item| item.downcast().ok())
        }

        pub fn iter(&self) -> impl Iterator<Item = T> {
            let s = self.clone();
            (0..).map_while(move |idx| s.get(idx))
        }

        pub fn append(&self, item: &T) {
            self.0.append(item);
        }
    }

    impl<T: IsA<Object>, L: IsA<ListStore>> From<L> for TypedListStore<T> {
        fn from(store: L) -> Self {
            Self(store.upcast(), std::marker::PhantomData)
        }
    }

    impl<T: IsA<Object>> Deref for TypedListStore<T> {
        type Target = ListStore;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T: IsA<Object>> DerefMut for TypedListStore<T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl<T: IsA<Object>> Default for TypedListStore<T> {
        fn default() -> Self {
            Self::new()
        }
    }
}
