use crate::{
    registry::Component,
    storage::{AbstractStorage, SparseSet},
};
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use std::{any::TypeId, collections::HashMap, hint::unreachable_unchecked};

#[derive(Default)]
pub struct Storages {
    storages: HashMap<TypeId, AtomicRefCell<Box<dyn AbstractStorage>>>,
}

impl Storages {
    pub fn register<T>(&mut self)
    where
        T: Component,
    {
        self.storages
            .entry(TypeId::of::<T>())
            .or_insert_with(|| AtomicRefCell::new(Box::new(SparseSet::<T>::default())));
    }

    pub fn borrow<T>(&self) -> Option<AtomicRef<SparseSet<T>>>
    where
        T: Component,
    {
        self.borrow_raw(TypeId::of::<T>()).map(|s| {
            AtomicRef::map(s, |s| match s.as_any().downcast_ref::<SparseSet<T>>() {
                Some(s) => s,
                None => unsafe { unreachable_unchecked() },
            })
        })
    }

    pub fn borrow_mut<T>(&self) -> Option<AtomicRefMut<SparseSet<T>>>
    where
        T: Component,
    {
        self.borrow_raw_mut(TypeId::of::<T>()).map(|s| {
            AtomicRefMut::map(s, |s| match s.as_mut_any().downcast_mut::<SparseSet<T>>() {
                Some(s) => s,
                None => unsafe { unreachable_unchecked() },
            })
        })
    }

    pub fn borrow_raw(&self, component: TypeId) -> Option<AtomicRef<dyn AbstractStorage>> {
        self.storages
            .get(&component)
            .map(|s| AtomicRef::map(s.borrow(), |s| s.as_ref()))
    }

    pub fn borrow_raw_mut(&self, component: TypeId) -> Option<AtomicRefMut<dyn AbstractStorage>> {
        self.storages
            .get(&component)
            .map(|s| AtomicRefMut::map(s.borrow_mut(), |s| s.as_mut()))
    }

    pub unsafe fn get_unchecked<T>(&self) -> Option<&SparseSet<T>>
    where
        T: Component,
    {
        self.storages.get(&TypeId::of::<T>()).map(|s| {
            match (*s.as_ptr()).as_any().downcast_ref::<SparseSet<T>>() {
                Some(s) => s,
                None => unreachable_unchecked(),
            }
        })
    }

    pub unsafe fn get_raw_unchecked(&self, component: TypeId) -> Option<&dyn AbstractStorage> {
        self.storages
            .get(&component)
            .map(|s| (*s.as_ptr()).as_ref())
    }

    pub unsafe fn get_mut_unchecked<T>(&self) -> Option<&mut SparseSet<T>>
    where
        T: Component,
    {
        self.storages.get(&TypeId::of::<T>()).map(|s| {
            match (*s.as_ptr()).as_mut_any().downcast_mut::<SparseSet<T>>() {
                Some(s) => s,
                None => unreachable_unchecked(),
            }
        })
    }

    pub unsafe fn get_mut_raw_unchecked(
        &self,
        component: TypeId,
    ) -> Option<&mut dyn AbstractStorage> {
        self.storages
            .get(&component)
            .map(|s| (*s.as_ptr()).as_mut())
    }
}
