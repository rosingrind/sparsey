use crate::entity::{
    group, ungroup, ungroup_all, Comp, CompMut, Component, ComponentSparseSet, Entity, Group,
    GroupInfo, GroupLayout, GroupMask, GroupMetadata, QueryMask, StorageMask,
};
use atomic_refcell::AtomicRefCell;
use rustc_hash::FxHashMap;
use std::any;
use std::any::TypeId;
use std::collections::hash_map::Entry;

#[derive(Default, Debug)]
pub struct ComponentStorage {
    groups: Vec<Group>,
    metadata: FxHashMap<TypeId, ComponentMetadata>,
    components: Vec<AtomicRefCell<ComponentSparseSet>>,
}

impl ComponentStorage {
    #[must_use]
    pub fn new(layout: &GroupLayout) -> Self {
        let mut groups = Vec::new();
        let mut metadata = FxHashMap::default();
        let mut components = Vec::new();

        for family in layout.families() {
            let storage_start = components.len();
            let group_start = groups.len();
            let group_end = group_start + family.arities().len();

            let mut prev_arity = 0;

            for &arity in family.arities() {
                let storage_end = storage_start + arity;
                let new_group_start = groups.len();

                groups.push(Group {
                    metadata: GroupMetadata {
                        storage_start,
                        new_storage_start: storage_start + prev_arity,
                        storage_end,
                        skip_mask: GroupMask::skip_from_to(new_group_start, group_end),
                        include_mask: QueryMask::include(arity),
                        exclude_mask: QueryMask::exclude(prev_arity, arity),
                    },
                    len: 0,
                });

                for local_storage_index in prev_arity..arity {
                    let component = &family.components()[local_storage_index];

                    metadata.insert(
                        component.type_id(),
                        ComponentMetadata {
                            storage_index: components.len(),
                            insert_mask: GroupMask::from_to(group_start, group_end),
                            delete_mask: GroupMask::from_to(new_group_start, group_end),
                            group_end: groups.len(),
                            storage_mask: StorageMask::single(local_storage_index),
                        },
                    );

                    components.push(AtomicRefCell::new(component.create_sparse_set()));
                }

                prev_arity = arity;
            }
        }

        Self {
            groups,
            metadata,
            components,
        }
    }

    pub fn register<T>(&mut self) -> bool
    where
        T: Component,
    {
        let Entry::Vacant(entry) = self.metadata.entry(TypeId::of::<T>()) else {
            return false;
        };

        entry.insert(ComponentMetadata {
            storage_index: self.components.len(),
            insert_mask: GroupMask::default(),
            delete_mask: GroupMask::default(),
            group_end: 0,
            storage_mask: StorageMask(0),
        });

        self.components
            .push(AtomicRefCell::new(ComponentSparseSet::new::<T>()));

        true
    }

    #[must_use]
    pub fn is_registered<T>(&self) -> bool
    where
        T: Component,
    {
        self.metadata.contains_key(&TypeId::of::<T>())
    }

    pub fn insert<C>(&mut self, entity: Entity, components: C)
    where
        C: ComponentSet,
    {
        C::insert(self, entity, components);
    }

    pub fn remove<C>(&mut self, entity: Entity) -> C::Remove
    where
        C: ComponentSet,
    {
        C::remove(self, entity)
    }

    pub fn delete<C>(&mut self, entity: Entity)
    where
        C: ComponentSet,
    {
        C::delete(self, entity);
    }

    pub fn delete_all(&mut self, entity: Entity) {
        unsafe {
            ungroup_all(&mut self.components, &mut self.groups, entity);
        }

        for sparse_set in &mut self.components {
            sparse_set.get_mut().delete_dyn(entity);
        }
    }

    #[must_use]
    pub fn borrow<T>(&self) -> Comp<T>
    where
        T: Component,
    {
        let Some(metadata) = self.metadata.get(&TypeId::of::<T>()) else {
            panic_missing_comp::<T>();
        };

        let group_info = (metadata.storage_mask.0 != 0).then(|| unsafe {
            GroupInfo::new(
                self.groups.get_unchecked(0..metadata.group_end),
                metadata.storage_mask,
            )
        });

        unsafe {
            Comp::new(
                self.components
                    .get_unchecked(metadata.storage_index)
                    .borrow(),
                group_info,
            )
        }
    }

    #[must_use]
    pub fn borrow_mut<T>(&self) -> CompMut<T>
    where
        T: Component,
    {
        let Some(metadata) = self.metadata.get(&TypeId::of::<T>()) else {
            panic_missing_comp::<T>();
        };

        let group_info = (metadata.storage_mask.0 != 0).then(|| unsafe {
            GroupInfo::new(
                self.groups.get_unchecked(0..metadata.group_end),
                metadata.storage_mask,
            )
        });

        unsafe {
            CompMut::new(
                self.components
                    .get_unchecked(metadata.storage_index)
                    .borrow_mut(),
                group_info,
            )
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ComponentMetadata {
    storage_index: usize,
    insert_mask: GroupMask,
    delete_mask: GroupMask,
    group_end: usize,
    storage_mask: StorageMask,
}

#[cold]
#[inline(never)]
fn panic_missing_comp<T>() -> ! {
    panic!("Component '{}' was not registered", any::type_name::<T>());
}

pub unsafe trait ComponentSet {
    type Remove;

    fn insert(storage: &mut ComponentStorage, entity: Entity, components: Self);

    #[must_use]
    fn remove(storage: &mut ComponentStorage, entity: Entity) -> Self::Remove;

    fn delete(storage: &mut ComponentStorage, entity: Entity);
}

macro_rules! impl_component_set {
    ($(($Comp:ident, $idx:tt)),*) => {
        unsafe impl<$($Comp,)*> ComponentSet for ($($Comp,)*)
        where
            $($Comp: Component,)*
        {
            type Remove = ($(Option<$Comp>,)*);

            fn insert(storage: &mut ComponentStorage, entity: Entity, components: Self) {
                let mut group_mask = GroupMask(0);

                $({
                    let metadata = storage.metadata
                        .get(&TypeId::of::<$Comp>())
                        .unwrap_or_else(|| panic_missing_comp::<$Comp>());

                    group_mask |= metadata.insert_mask;

                    unsafe {
                        storage
                            .components
                            .get_unchecked_mut(metadata.storage_index)
                            .get_mut()
                            .insert(entity, components.$idx);
                    }
                })*

                unsafe {
                    group(
                        &mut storage.components,
                        &mut storage.groups,
                        group_mask,
                        entity,
                    );
                }
            }

            fn remove(storage: &mut ComponentStorage, entity: Entity) -> Self::Remove {
                let mut group_mask = GroupMask(0);

                let indexes = ($({
                    let metadata = storage.metadata
                        .get(&TypeId::of::<$Comp>())
                        .unwrap_or_else(|| panic_missing_comp::<$Comp>());

                    group_mask |= metadata.delete_mask;
                    metadata.storage_index
                },)*);

                unsafe {
                    ungroup(
                        &mut storage.components,
                        &mut storage.groups,
                        group_mask,
                        entity,
                    );

                    ($(
                        storage
                            .components
                            .get_unchecked_mut(indexes.$idx)
                            .get_mut()
                            .remove::<$Comp>(entity),
                    )*)
                }
            }

            fn delete(storage: &mut ComponentStorage, entity: Entity) {
                let mut group_mask = GroupMask(0);

                let indexes = ($({
                    let metadata = storage.metadata
                        .get(&TypeId::of::<$Comp>())
                        .unwrap_or_else(|| panic_missing_comp::<$Comp>());

                    group_mask |= metadata.delete_mask;
                    metadata.storage_index
                },)*);

                unsafe {
                    ungroup(
                        &mut storage.components,
                        &mut storage.groups,
                        group_mask,
                        entity,
                    );

                    $(
                        storage
                            .components
                            .get_unchecked_mut(indexes.$idx)
                            .get_mut()
                            .delete::<$Comp>(entity);
                    )*
                }
            }
        }
    };
}

#[allow(unused_variables)]
unsafe impl ComponentSet for () {
    type Remove = ();

    #[inline(always)]
    fn insert(storage: &mut ComponentStorage, entity: Entity, components: Self) {
        // Empty
    }

    #[inline(always)]
    fn remove(storage: &mut ComponentStorage, entity: Entity) -> Self::Remove {
        // Empty
    }

    #[inline(always)]
    fn delete(storage: &mut ComponentStorage, entity: Entity) {
        // Empty
    }
}

#[rustfmt::skip]
mod impls {
    use super::*;

    impl_component_set!((A, 0));
    impl_component_set!((A, 0), (B, 1));
    impl_component_set!((A, 0), (B, 1), (C, 2));
    impl_component_set!((A, 0), (B, 1), (C, 2), (D, 3));
    impl_component_set!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4));
    impl_component_set!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5));
    impl_component_set!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6));
    impl_component_set!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7));
    impl_component_set!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8));
    impl_component_set!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9));
    impl_component_set!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10));
    impl_component_set!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11));
    impl_component_set!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11), (M, 12));
    impl_component_set!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11), (M, 12), (N, 13));
    impl_component_set!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11), (M, 12), (N, 13), (O, 14));
    impl_component_set!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11), (M, 12), (N, 13), (O, 14), (P, 15));
}
