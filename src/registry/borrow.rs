use crate::{
    data::view::*,
    entity::{Entity, IndexEntity},
    registry::{Component, World},
    storage::{SparseArray, SparseSet},
};
use atomic_refcell::{AtomicRef, AtomicRefMut};

pub trait BorrowFromWorld<'a> {
    fn borrow(world: &'a World) -> Self;
}

pub struct Comp<'a, T> {
    set: AtomicRef<'a, SparseSet<T>>,
}

impl<'a, T> BorrowFromWorld<'a> for Comp<'a, T>
where
    T: Component,
{
    fn borrow(world: &'a World) -> Self {
        Self {
            set: world.borrow().unwrap(),
        }
    }
}

impl<'a, T> StorageView<'a> for &'a Comp<'a, T> {
    const STRICT: bool = true;
    type Output = &'a T;
    type Component = &'a T;
    type Data = *const T;

    unsafe fn split_for_iteration(self) -> (&'a SparseArray, &'a [Entity], Self::Data) {
        <&'a SparseSet<T> as StorageView<'a>>::split_for_iteration(&self.set)
    }

    unsafe fn get_output(self, entity: Entity) -> Option<Self::Output> {
        <&'a SparseSet<T> as StorageView<'a>>::get_output(&self.set, entity)
    }

    unsafe fn get_component(data: Self::Data, entity: IndexEntity) -> Self::Component {
        <&'a SparseSet<T> as StorageView<'a>>::get_component(data, entity)
    }

    unsafe fn get_from_component(component: Option<Self::Component>) -> Option<Self::Output> {
        <&'a SparseSet<T> as StorageView<'a>>::get_from_component(component)
    }
}

pub struct CompMut<'a, T> {
    set: AtomicRefMut<'a, SparseSet<T>>,
}

impl<'a, T> StorageView<'a> for &'a CompMut<'a, T> {
    const STRICT: bool = true;
    type Output = &'a T;
    type Component = &'a T;
    type Data = *const T;

    unsafe fn split_for_iteration(self) -> (&'a SparseArray, &'a [Entity], Self::Data) {
        <&'a SparseSet<T> as StorageView<'a>>::split_for_iteration(&self.set)
    }

    unsafe fn get_output(self, entity: Entity) -> Option<Self::Output> {
        <&'a SparseSet<T> as StorageView<'a>>::get_output(&self.set, entity)
    }

    unsafe fn get_component(data: Self::Data, entity: IndexEntity) -> Self::Component {
        <&'a SparseSet<T> as StorageView<'a>>::get_component(data, entity)
    }

    unsafe fn get_from_component(component: Option<Self::Component>) -> Option<Self::Output> {
        <&'a SparseSet<T> as StorageView<'a>>::get_from_component(component)
    }
}

impl<'a, 'b, T> StorageView<'a> for &'a mut CompMut<'b, T>
where
    'b: 'a,
{
    const STRICT: bool = true;
    type Output = &'a mut T;
    type Component = &'a mut T;
    type Data = *mut T;

    unsafe fn split_for_iteration(self) -> (&'a SparseArray, &'a [Entity], Self::Data) {
        <&'a mut SparseSet<T> as StorageView<'a>>::split_for_iteration(&mut *self.set)
    }

    unsafe fn get_output(self, entity: Entity) -> Option<Self::Output> {
        <&'a mut SparseSet<T> as StorageView<'a>>::get_output(&mut *self.set, entity)
    }

    unsafe fn get_component(data: Self::Data, entity: IndexEntity) -> Self::Component {
        <&'a mut SparseSet<T> as StorageView<'a>>::get_component(data, entity)
    }

    unsafe fn get_from_component(component: Option<Self::Component>) -> Option<Self::Output> {
        <&'a mut SparseSet<T> as StorageView<'a>>::get_from_component(component)
    }
}

impl<'a, T> BorrowFromWorld<'a> for CompMut<'a, T>
where
    T: Component,
{
    fn borrow(world: &'a World) -> Self {
        Self {
            set: world.borrow_mut().unwrap(),
        }
    }
}

pub struct RawViewMut<'a, T> {
    pub set: AtomicRefMut<'a, SparseSet<T>>,
}

impl<'a, T> BorrowFromWorld<'a> for RawViewMut<'a, T>
where
    T: Component,
{
    fn borrow(world: &'a World) -> Self {
        Self {
            set: world.borrow_raw_mut().unwrap(),
        }
    }
}

macro_rules! impl_borrow_from_world {
    ($($b:ident),+) => {
        impl<'a, $($b,)+> BorrowFromWorld<'a> for ($($b,)+)
        where
            $($b: BorrowFromWorld<'a>,)+
        {
            fn borrow(world: &'a World) -> Self {
                ($(<$b as BorrowFromWorld<'a>>::borrow(world),)+)
            }
        }
    };
}

impl_borrow_from_world!(A);
impl_borrow_from_world!(A, B);
impl_borrow_from_world!(A, B, C);
impl_borrow_from_world!(A, B, C, D);
impl_borrow_from_world!(A, B, C, D, E);
impl_borrow_from_world!(A, B, C, D, E, F);
impl_borrow_from_world!(A, B, C, D, E, F, G);
impl_borrow_from_world!(A, B, C, D, E, F, G, H);
