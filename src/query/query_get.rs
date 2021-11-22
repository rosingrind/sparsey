use crate::components::{CombinedGroupInfo, Component, GroupInfo};
use crate::query::{IterData, Passthrough};
use crate::storage::{Entity, EntitySparseArray};
use crate::utils::{ChangeTicks, Ticks};

pub trait ChangeTicksFilter
where
    Self: 'static,
{
    fn matches(ticks: &ChangeTicks, world_tick: Ticks, change_tick: Ticks) -> bool;
}

impl ChangeTicksFilter for Passthrough {
    #[inline(always)]
    fn matches(_: &ChangeTicks, _: Ticks, _: Ticks) -> bool {
        true
    }
}

#[derive(Clone, Copy)]
pub struct ComponentViewData<T> {
    pub components: *mut T,
    pub ticks: *mut ChangeTicks,
}

impl<T> ComponentViewData<T> {
    pub const fn new(components: *mut T, ticks: *mut ChangeTicks) -> Self {
        Self { components, ticks }
    }
}

pub trait GetComponent<'a> {
    type Item: 'a;
    type Component: Component;

    fn group_info(&self) -> Option<GroupInfo<'a>>;

    fn change_detection_ticks(&self) -> (Ticks, Ticks);

    fn get_index(&self, entity: Entity) -> Option<usize>;

    unsafe fn get_unchecked<F>(self, index: usize) -> Option<Self::Item>;

    fn split(
        self,
    ) -> (
        &'a [Entity],
        &'a EntitySparseArray,
        *mut Self::Component,
        *mut ChangeTicks,
    );

    unsafe fn get_from_parts_unchecked<F>(
        components: *mut Self::Component,
        ticks: *mut ChangeTicks,
        index: usize,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item>
    where
        F: ChangeTicksFilter;
}

pub trait GetComponentSet<'a> {
    type Item: 'a;
    type Index: Copy;
    type Sparse: 'a;
    type Data;

    fn group_info(&self) -> Option<CombinedGroupInfo<'a>>;

    fn change_detection_ticks(&self) -> (Ticks, Ticks);

    fn get_index(&self, entity: Entity) -> Option<Self::Index>;

    unsafe fn get_unchecked<F>(self, index: Self::Index) -> Option<Self::Item>
    where
        F: ChangeTicksFilter;

    fn split_sparse(self) -> (IterData<'a>, Self::Sparse, Self::Data);

    fn split_dense(self) -> (IterData<'a>, Self::Data);

    fn get_index_from_sparse(sparse: &Self::Sparse, entity: Entity) -> Option<Self::Index>;

    unsafe fn get_sparse_unchecked<F>(
        data: &Self::Data,
        index: Self::Index,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item>
    where
        F: ChangeTicksFilter;

    unsafe fn get_dense_unchecked<F>(
        data: &Self::Data,
        index: usize,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item>
    where
        F: ChangeTicksFilter;
}

impl<'a, G> GetComponentSet<'a> for G
where
    G: GetComponent<'a>,
{
    type Item = G::Item;
    type Index = usize;
    type Sparse = &'a EntitySparseArray;
    type Data = ComponentViewData<G::Component>;

    fn group_info(&self) -> Option<CombinedGroupInfo<'a>> {
        CombinedGroupInfo::default().combine(GetComponent::group_info(self)?)
    }

    fn change_detection_ticks(&self) -> (Ticks, Ticks) {
        GetComponent::change_detection_ticks(self)
    }

    fn get_index(&self, entity: Entity) -> Option<Self::Index> {
        GetComponent::get_index(self, entity)
    }

    unsafe fn get_unchecked<F>(self, index: Self::Index) -> Option<Self::Item>
    where
        F: ChangeTicksFilter,
    {
        GetComponent::get_unchecked::<F>(self, index)
    }

    fn split_sparse(self) -> (IterData<'a>, Self::Sparse, Self::Data) {
        let (world_tick, change_tick) = GetComponent::change_detection_ticks(&self);
        let (entities, sparse, components, ticks) = GetComponent::split(self);

        (
            IterData::new(entities, world_tick, change_tick),
            sparse,
            ComponentViewData::new(components, ticks),
        )
    }

    fn split_dense(self) -> (IterData<'a>, Self::Data) {
        let (world_tick, change_tick) = GetComponent::change_detection_ticks(&self);
        let (entities, _, components, ticks) = GetComponent::split(self);

        (
            IterData::new(entities, world_tick, change_tick),
            ComponentViewData::new(components, ticks),
        )
    }

    fn get_index_from_sparse(sparse: &Self::Sparse, entity: Entity) -> Option<Self::Index> {
        sparse.get_entity(entity).map(|e| e.dense())
    }

    unsafe fn get_sparse_unchecked<F>(
        data: &Self::Data,
        index: Self::Index,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item>
    where
        F: ChangeTicksFilter,
    {
        G::get_from_parts_unchecked::<F>(
            data.components,
            data.ticks,
            index,
            world_tick,
            change_tick,
        )
    }

    unsafe fn get_dense_unchecked<F>(
        data: &Self::Data,
        index: usize,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item>
    where
        F: ChangeTicksFilter,
    {
        G::get_from_parts_unchecked::<F>(
            data.components,
            data.ticks,
            index,
            world_tick,
            change_tick,
        )
    }
}

pub trait GetComponentSetFiltered<'a> {
    type Item: 'a;
    type Filter: ChangeTicksFilter;
    type Index: Copy;
    type Sparse;
    type Data;

    fn group_info(&self) -> Option<CombinedGroupInfo<'a>>;

    fn change_detection_ticks(&self) -> (Ticks, Ticks);

    fn get_index(&self, entity: Entity) -> Option<Self::Index>;

    unsafe fn get_unchecked(self, index: Self::Index) -> Option<Self::Item>;

    fn split_sparse(self) -> (IterData<'a>, Self::Sparse, Self::Data);

    fn split_dense(self) -> (IterData<'a>, Self::Data);

    fn get_index_from_sparse(sparse: &Self::Sparse, entity: Entity) -> Option<Self::Index>;

    unsafe fn get_sparse_unchecked(
        data: &Self::Data,
        index: Self::Index,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item>;

    unsafe fn get_dense_unchecked(
        data: &Self::Data,
        index: usize,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item>;
}

impl<'a, G> GetComponentSetFiltered<'a> for G
where
    G: GetComponentSet<'a>,
{
    type Item = G::Item;
    type Filter = Passthrough;
    type Index = G::Index;
    type Sparse = G::Sparse;
    type Data = G::Data;

    fn group_info(&self) -> Option<CombinedGroupInfo<'a>> {
        GetComponentSet::group_info(self)
    }

    fn change_detection_ticks(&self) -> (Ticks, Ticks) {
        GetComponentSet::change_detection_ticks(self)
    }

    fn get_index(&self, entity: Entity) -> Option<Self::Index> {
        GetComponentSet::get_index(self, entity)
    }

    unsafe fn get_unchecked(self, index: Self::Index) -> Option<Self::Item> {
        GetComponentSet::get_unchecked::<Self::Filter>(self, index)
    }

    fn split_sparse(self) -> (IterData<'a>, Self::Sparse, Self::Data) {
        GetComponentSet::split_sparse(self)
    }

    fn split_dense(self) -> (IterData<'a>, Self::Data) {
        GetComponentSet::split_dense(self)
    }

    fn get_index_from_sparse(sparse: &Self::Sparse, entity: Entity) -> Option<Self::Index> {
        G::get_index_from_sparse(sparse, entity)
    }

    unsafe fn get_sparse_unchecked(
        data: &Self::Data,
        index: Self::Index,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item> {
        G::get_sparse_unchecked::<Self::Filter>(data, index, world_tick, change_tick)
    }

    unsafe fn get_dense_unchecked(
        data: &Self::Data,
        index: usize,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item> {
        G::get_dense_unchecked::<Self::Filter>(data, index, world_tick, change_tick)
    }
}

pub trait QueryGet<'a> {
    type Item;
    type Sparse;
    type Data;

    fn group_info(&self) -> Option<CombinedGroupInfo<'a>>;

    fn change_detection_ticks(&self) -> (Ticks, Ticks);

    fn get(self, entity: Entity) -> Option<Self::Item>;

    fn split_sparse(self) -> (IterData<'a>, Self::Sparse, Self::Data);

    fn split_dense(self) -> (IterData<'a>, Self::Data);

    unsafe fn get_sparse_unchecked(
        sparse: &'a Self::Sparse,
        entity: Entity,
        data: &Self::Data,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item>;

    unsafe fn get_dense_unchecked(
        data: &Self::Data,
        index: usize,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item>;
}

impl<'a, G> QueryGet<'a> for G
where
    G: GetComponentSetFiltered<'a>,
{
    type Item = G::Item;
    type Sparse = G::Sparse;
    type Data = G::Data;

    fn group_info(&self) -> Option<CombinedGroupInfo<'a>> {
        GetComponentSetFiltered::group_info(self)
    }

    fn change_detection_ticks(&self) -> (Ticks, Ticks) {
        GetComponentSetFiltered::change_detection_ticks(self)
    }

    fn get(self, entity: Entity) -> Option<Self::Item> {
        let index = GetComponentSetFiltered::get_index(&self, entity)?;
        unsafe { GetComponentSetFiltered::get_unchecked(self, index) }
    }

    fn split_sparse(self) -> (IterData<'a>, Self::Sparse, Self::Data) {
        GetComponentSetFiltered::split_sparse(self)
    }

    fn split_dense(self) -> (IterData<'a>, Self::Data) {
        GetComponentSetFiltered::split_dense(self)
    }

    unsafe fn get_sparse_unchecked(
        sparse: &'a Self::Sparse,
        entity: Entity,
        data: &Self::Data,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item> {
        let index = G::get_index_from_sparse(sparse, entity)?;
        G::get_sparse_unchecked(data, index, world_tick, change_tick)
    }

    unsafe fn get_dense_unchecked(
        data: &Self::Data,
        index: usize,
        world_tick: Ticks,
        change_tick: Ticks,
    ) -> Option<Self::Item> {
        G::get_dense_unchecked(data, index, world_tick, change_tick)
    }
}
