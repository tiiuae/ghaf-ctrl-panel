use gio::{ListModel, ListStore};
use glib::Object;
use gtk::{self, gio, glib, prelude::*, CustomFilter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

pub mod imp {
    use super::*;

    pub trait TypedModelExt<T: IsA<Object>> {
        fn model(&self) -> ListModel;

        fn get(&self, index: u32) -> Option<T> {
            self.model()
                .item(index)
                .and_then(|item| item.downcast().ok())
        }

        fn iter(&self) -> impl Iterator<Item = T> {
            (0..).map_while(move |idx| self.get(idx))
        }

        fn typed_iter(&self) -> impl Iterator<Item = T> {
            self.iter()
        }
    }

    impl<T: IsA<Object>, M: IsA<ListModel>> TypedModelExt<T> for M {
        fn model(&self) -> ListModel {
            self.clone().upcast()
        }
    }

    #[derive(Debug, Clone)]
    pub struct TypedListModel<T: IsA<Object>>(ListModel, PhantomData<T>);

    impl<T: IsA<Object>> TypedModelExt<T> for TypedListModel<T> {
        fn model(&self) -> ListModel {
            self.0.clone()
        }
    }

    #[derive(Debug, Clone)]
    pub struct TypedListStore<T: IsA<Object>>(ListStore, PhantomData<T>);

    impl<T: IsA<Object>> TypedModelExt<T> for TypedListStore<T> {
        fn model(&self) -> ListModel {
            self.0.clone().upcast()
        }
    }

    impl<T: IsA<Object>> TypedListStore<T> {
        pub fn new() -> Self {
            Self(ListStore::new::<T>(), PhantomData)
        }
    }

    impl<T: IsA<Object>, L: IsA<ListStore>> From<L> for TypedListStore<T> {
        fn from(store: L) -> Self {
            Self(store.upcast(), PhantomData)
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

    impl<T: IsA<Object>> Extend<T> for TypedListStore<T> {
        fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
            self.0.extend(iter)
        }
    }

    pub struct TypedCustomFilter();

    impl TypedCustomFilter {
        pub fn new<F: Fn(&T) -> bool + 'static, T: IsA<Object>>(f: F) -> CustomFilter {
            CustomFilter::new(move |obj| obj.downcast_ref().is_some_and(&f))
        }
    }
}
