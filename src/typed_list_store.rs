#![allow(dead_code)]
use gio::{ListModel, ListStore};
use glib::{Object, SignalHandlerId};
use gtk::{self, ListItem, SignalListItemFactory, SingleSelection, Widget, gio, glib, prelude::*};
use std::ops::Deref;

#[allow(dead_code)]
pub trait TypedListModelExt<T: IsA<Object>>: IsA<ListModel> {
    fn model(&self) -> &ListModel {
        self.upcast_ref()
    }

    fn get(&self, index: u32) -> Option<T> {
        self.model().item(index).and_downcast()
    }

    fn typed_iter(&self) -> impl Iterator<Item = T> {
        (0..).map_while(move |idx| self.get(idx))
    }
}

impl<T: IsA<Object>, L: IsA<ListModel> + Clone> TypedListModelExt<T> for L {}

#[allow(dead_code)]
pub trait TypedListWrapperExt: IsA<ListModel> {
    fn wrap<T: IsA<Object>>(self) -> TypedListWrapper<Self, T> {
        TypedListWrapper::<Self, T>::from(self)
    }
}

impl<L: IsA<ListModel>> TypedListWrapperExt for L {}

#[allow(dead_code)]
pub trait SingleSelectionExt: IsA<SingleSelection> {
    fn selected_obj<T: IsA<Object>>(&self) -> Option<T> {
        self.upcast_ref().selected_item().and_downcast()
    }
}

impl<L: IsA<SingleSelection>> SingleSelectionExt for L {}

pub trait TypedSelectionWrapperExt<T: IsA<Object>> {
    fn selected_obj(&self) -> Option<T>;
}

impl<T: IsA<Object>, L: IsA<SingleSelection> + IsA<ListModel>> TypedSelectionWrapperExt<T>
    for TypedListWrapper<L, T>
{
    fn selected_obj(&self) -> Option<T> {
        SingleSelectionExt::selected_obj::<T>(&**self)
    }
}

impl<T: IsA<Object>> Extend<T> for TypedListWrapper<ListStore, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, i: I) {
        self.0.extend(i);
    }
}

#[derive(Debug, Clone)]
pub struct TypedListWrapper<L: IsA<ListModel>, T: IsA<Object>>(
    pub(super) L,
    std::marker::PhantomData<T>,
);

impl<L: IsA<ListModel>, T: IsA<Object>> TypedListWrapper<L, T> {
    pub fn get(&self, index: u32) -> Option<T> {
        TypedListModelExt::<T>::get(&self.0, index)
    }

    pub fn iter(&self) -> impl Iterator<Item = T> {
        let this = self.clone();
        (0..).map_while(move |idx| this.get(idx))
    }
}

impl<T: IsA<Object>, L: IsA<ListModel>> From<L> for TypedListWrapper<L, T> {
    fn from(store: L) -> Self {
        Self(store, std::marker::PhantomData)
    }
}

impl<T: IsA<Object>, L: IsA<ListModel>> Deref for TypedListWrapper<L, T> {
    type Target = L;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, S> std::iter::FromIterator<T> for TypedListWrapper<S, T>
where
    T: IsA<Object>,
    S: std::iter::FromIterator<T> + IsA<ListModel>,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter().collect::<S>().wrap()
    }
}

pub type TypedListStore<T> = TypedListWrapper<ListStore, T>;
pub type TypedListModel<T> = TypedListWrapper<ListModel, T>;

impl<T: IsA<Object>> From<TypedListStore<T>> for TypedListModel<T> {
    fn from(store: TypedListStore<T>) -> Self {
        store.0.upcast::<ListModel>().wrap()
    }
}

#[derive(Clone)]
pub struct TypedSignalListItemFactory<I: IsA<Object>, W: IsA<Widget>>(
    SignalListItemFactory,
    std::marker::PhantomData<I>,
    std::marker::PhantomData<W>,
);

#[allow(dead_code)]
impl<I: IsA<Object>, W: IsA<Widget>> TypedSignalListItemFactory<I, W> {
    pub fn new() -> Self {
        Self(
            SignalListItemFactory::new(),
            std::marker::PhantomData,
            std::marker::PhantomData,
        )
    }

    pub fn on_setup<F: Fn(&Self) -> W + 'static>(&self, f: F) {
        let this = self.clone();
        self.0.on_setup(move |_, item| {
            item.set_child(Some(&f(&this)));
        });
    }

    pub fn on_bind<F: Fn(&Self, &W, &I) + 'static>(&self, f: F) {
        let this = self.clone();
        self.0.on_bind(move |_, item, itm| {
            if let Some(wdg) = item.child().and_downcast_ref() {
                f(&this, wdg, itm);
            }
        });
    }

    pub fn on_unbind<F: Fn(&Self, &W) + 'static>(&self, f: F) {
        let this = self.clone();
        self.0.on_unbind(move |_, item| {
            if let Some(wdg) = item.child().and_downcast_ref() {
                f(&this, wdg);
            }
        });
    }

    pub fn on_teardown<F: Fn(&Self, &W) + 'static>(self, f: F) {
        let this = self.clone();
        self.0.on_teardown(move |_, item| {
            if let Some(wdg) = item.child().and_downcast_ref() {
                f(&this, wdg);
            }
            item.set_child(Widget::NONE);
        });
    }
}

impl<I: IsA<Object>, W: IsA<Widget>> Deref for TypedSignalListItemFactory<I, W> {
    type Target = SignalListItemFactory;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[allow(dead_code)]
pub trait SignalListItemFactoryExt {
    fn on_setup<F: Fn(&Self, &ListItem) + 'static>(&self, f: F) -> SignalHandlerId;
    fn on_bind<I: IsA<Object>, F: Fn(&Self, &ListItem, &I) + 'static>(
        &self,
        f: F,
    ) -> SignalHandlerId;
    fn on_teardown<F: Fn(&Self, &ListItem) + 'static>(&self, f: F) -> SignalHandlerId;
    fn on_unbind<F: Fn(&Self, &ListItem) + 'static>(&self, f: F) -> SignalHandlerId;
}

impl SignalListItemFactoryExt for SignalListItemFactory {
    fn on_setup<F: Fn(&Self, &ListItem) + 'static>(&self, f: F) -> SignalHandlerId {
        self.connect_setup(move |this, obj| f(this, unsafe { obj.unsafe_cast_ref() }))
    }

    fn on_bind<I: IsA<Object>, F: Fn(&Self, &ListItem, &I) + 'static>(
        &self,
        f: F,
    ) -> SignalHandlerId {
        self.connect_bind(move |this, obj| {
            let litem = unsafe { obj.unsafe_cast_ref() };
            f(
                this,
                litem,
                litem
                    .item()
                    .and_downcast_ref()
                    .expect("List item must have item"),
            );
        })
    }

    fn on_teardown<F: Fn(&Self, &ListItem) + 'static>(&self, f: F) -> SignalHandlerId {
        self.connect_teardown(move |this, obj| f(this, unsafe { obj.unsafe_cast_ref() }))
    }

    fn on_unbind<F: Fn(&Self, &ListItem) + 'static>(&self, f: F) -> SignalHandlerId {
        self.connect_unbind(move |this, obj| f(this, unsafe { obj.unsafe_cast_ref() }))
    }
}
