use crate::components::{Component, ComponentStorage, Entity};
use crate::layout::Layout;
use crate::resources::{Resource, Resources};
use crate::utils::{
	panic_missing_comp, panic_missing_res, ChangeTicks, FetchFrom, NonZeroTicks, Ticks,
};
use crate::world::{
	BorrowStorages, Comp, CompMut, ComponentSet, ComponentStorages, EntityStorage, NoSuchEntity,
	Res, ResMut, TickOverflow,
};
use std::any::TypeId;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

/// Uniquely identifies a `World` during the execution of the program.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct WorldId(NonZeroU64);

impl WorldId {
	fn new() -> Self {
		static COUNTER: AtomicU64 = AtomicU64::new(1);

		let id = COUNTER.fetch_add(1, Ordering::Relaxed);
		NonZeroU64::new(id).map(Self).expect("Ran out of WorldIds")
	}
}

/// Container for component storages and entities.
pub struct World {
	id: WorldId,
	tick: NonZeroTicks,
	entities: EntityStorage,
	storages: ComponentStorages,
	resources: Resources,
}

impl Default for World {
	fn default() -> Self {
		Self {
			id: WorldId::new(),
			tick: NonZeroTicks::new(1).unwrap(),
			entities: Default::default(),
			storages: Default::default(),
			resources: Default::default(),
		}
	}
}

impl World {
	/// Creates an empty world with the storages arranged as described by
	/// `layout`.
	pub fn with_layout(layout: &Layout) -> Self {
		let mut world = Self::default();
		world.set_layout(layout);
		world
	}

	/// Arranges the storages as described by `layout`. This function iterates
	/// through all the entities to ararange their components, so it is best
	/// called right after creating the `World`.
	pub fn set_layout(&mut self, layout: &Layout) {
		self.storages.set_layout(layout, self.entities.as_ref());
	}

	/// Creates a component storage for `T` if one doesn't already exist.
	pub fn register<T>(&mut self)
	where
		T: Component,
	{
		self.storages.register::<T>()
	}

	/// Creates an `Entity` with the given `components` and returns it.
	pub fn create<C>(&mut self, components: C) -> Entity
	where
		C: ComponentSet,
	{
		let ticks = ChangeTicks::just_added(self.tick.get());
		self.create_with_ticks(components, ticks)
	}

	/// Same as `create`, but the `ChangeTicks` are provided by the caller.
	pub fn create_with_ticks<C>(&mut self, components: C, ticks: ChangeTicks) -> Entity
	where
		C: ComponentSet,
	{
		let entity = self.entities.create();
		let _ = self.append_with_ticks(entity, components, ticks);
		entity
	}

	/// Creates new `Entities` with the components produced by
	/// `components_iter`. Returns the newly created entities as a slice.
	pub fn extend<C, I>(&mut self, components_iter: I) -> &[Entity]
	where
		C: ComponentSet,
		I: IntoIterator<Item = C>,
	{
		let ticks = ChangeTicks::just_added(self.tick.get());
		self.extend_with_ticks(components_iter, ticks)
	}

	/// Same as `extend`, but the `ChangeTicks` are provided by the caller.
	pub fn extend_with_ticks<C, I>(&mut self, components_iter: I, ticks: ChangeTicks) -> &[Entity]
	where
		C: ComponentSet,
		I: IntoIterator<Item = C>,
	{
		let initial_entity_count = self.entities.as_ref().len();

		let families = {
			let (mut storages, families) = C::Storages::borrow_with_families(&self.storages);
			let entities = &mut self.entities;

			components_iter.into_iter().for_each(|components| {
				let entity = entities.create();

				unsafe {
					C::insert(&mut storages, entity, components, ticks);
				}
			});

			families
		};

		let new_entities = &self.entities.as_ref()[initial_entity_count..];

		for i in families.indexes() {
			for &entity in new_entities {
				unsafe {
					self.storages.grouped.group_components(i, entity);
				}
			}
		}

		new_entities
	}

	/// Removes `entity` and all of its `components` from the world.
	/// Returns `true` if `entity` existed in the world before the call.
	pub fn destroy(&mut self, entity: Entity) -> bool {
		if !self.entities.destroy(entity) {
			return false;
		}

		for i in 0..self.storages.grouped.group_family_count() {
			unsafe {
				self.storages.grouped.ungroup_components(i, entity);
			}
		}

		for storage in self.storages.iter_storages_mut() {
			storage.remove_and_drop(entity);
		}

		true
	}

	/// Appends the given `components` to `entity` if `entity` exists in the
	/// `World`.
	pub fn append<C>(&mut self, entity: Entity, components: C) -> Result<(), NoSuchEntity>
	where
		C: ComponentSet,
	{
		let ticks = ChangeTicks::just_added(self.tick.get());
		self.append_with_ticks(entity, components, ticks)
	}

	/// Same as `append`, but the `ChangeTicks` are provided by the caller.
	pub fn append_with_ticks<C>(
		&mut self,
		entity: Entity,
		components: C,
		ticks: ChangeTicks,
	) -> Result<(), NoSuchEntity>
	where
		C: ComponentSet,
	{
		if !self.contains(entity) {
			return Err(NoSuchEntity);
		}

		let families = unsafe {
			let (mut storages, families) = C::Storages::borrow_with_families(&self.storages);
			C::insert(&mut storages, entity, components, ticks);
			families
		};

		for i in families.indexes() {
			unsafe {
				self.storages.grouped.group_components(i, entity);
			}
		}

		Ok(())
	}

	/// Removes a component set from `entity` and returns them if they all
	/// existed in the `World` before the call.
	pub fn remove<C>(&mut self, entity: Entity) -> Option<C>
	where
		C: ComponentSet,
	{
		if !self.contains(entity) {
			return None;
		}

		let families = C::Storages::families(&self.storages);

		for i in families.indexes() {
			unsafe {
				self.storages.grouped.ungroup_components(i, entity);
			}
		}

		unsafe {
			let mut storages = C::Storages::borrow(&self.storages);
			C::remove(&mut storages, entity)
		}
	}

	/// Deletes a component set from `entity`. This is faster than removing
	/// the components.
	pub fn delete<C>(&mut self, entity: Entity)
	where
		C: ComponentSet,
	{
		if !self.contains(entity) {
			return;
		}

		let families = C::Storages::families(&self.storages);

		for i in families.indexes() {
			unsafe {
				self.storages.grouped.ungroup_components(i, entity);
			}
		}

		unsafe {
			let mut storages = C::Storages::borrow(&self.storages);
			C::delete(&mut storages, entity);
		}
	}

	/// Returns `true` if `entity` exists in the `World`.
	pub fn contains(&self, entity: Entity) -> bool {
		self.entities.contains(entity)
	}

	/// Removes all entities and components in the world.
	pub fn clear(&mut self) {
		self.entities.clear();
		self.storages.clear();
	}

	/// Advances the current world tick. Should be called after each game
	/// update for proper change detection.
	pub fn advance_ticks(&mut self) -> Result<(), TickOverflow> {
		if self.tick.get() != Ticks::MAX {
			self.tick = NonZeroTicks::new(self.tick.get() + 1).unwrap();
			Ok(())
		} else {
			self.tick = NonZeroTicks::new(1).unwrap();
			Err(TickOverflow)
		}
	}

	/// Returns all the `entities` in the world as a slice.
	pub fn entities(&self) -> &[Entity] {
		self.entities.as_ref()
	}

	/// Returns the `WorldId` which uniquely identifies this `World`.
	pub fn id(&self) -> WorldId {
		self.id
	}

	/// Returns the current world tick used for change detection.
	pub fn tick(&self) -> Ticks {
		self.tick.get()
	}

	pub fn fetch<'a, T>(&'a self) -> T::Item
	where
		T: FetchFrom<'a, Self>,
	{
		T::fetch(self, self.tick.get() - 1)
	}

	pub(crate) unsafe fn register_storage(&mut self, component: TypeId, storage: ComponentStorage) {
		self.storages.register_storage(component, storage);
	}

	pub(crate) fn entity_storage(&self) -> &EntityStorage {
		&self.entities
	}

	pub(crate) fn component_storages(&self) -> &ComponentStorages {
		&self.storages
	}

	pub(crate) fn maintain(&mut self) {
		self.entities.maintain();
	}
}

impl<'a, 'b, T> FetchFrom<'a, World> for Comp<'b, T>
where
	T: Component,
{
	type Item = Comp<'a, T>;

	fn fetch(world: &'a World, change_tick: Ticks) -> Self::Item {
		let (storage, info) = world
			.storages
			.borrow_with_info(&TypeId::of::<T>())
			.unwrap_or_else(|| panic_missing_comp::<T>());

		unsafe { Comp::new(storage, info, world.tick.get(), change_tick) }
	}
}

impl<'a, 'b, T> FetchFrom<'a, World> for CompMut<'b, T>
where
	T: Component,
{
	type Item = CompMut<'a, T>;

	fn fetch(world: &'a World, change_tick: Ticks) -> Self::Item {
		let (storage, info) = world
			.storages
			.borrow_with_info_mut(&TypeId::of::<T>())
			.unwrap_or_else(|| panic_missing_comp::<T>());

		unsafe { CompMut::new(storage, info, world.tick.get(), change_tick) }
	}
}

impl<'a, 'b, T> FetchFrom<'a, World> for Res<'b, T>
where
	T: Resource,
{
	type Item = Res<'a, T>;

	fn fetch(world: &'a World, change_tick: Ticks) -> Self::Item {
		let cell = world
			.resources
			.borrow::<T>()
			.unwrap_or_else(|| panic_missing_res::<T>());

		unsafe { Res::new(cell, world.tick.get(), change_tick) }
	}
}

impl<'a, 'b, T> FetchFrom<'a, World> for ResMut<'b, T>
where
	T: Resource,
{
	type Item = ResMut<'a, T>;

	fn fetch(world: &'a World, change_tick: Ticks) -> Self::Item {
		let cell = world
			.resources
			.borrow_mut::<T>()
			.unwrap_or_else(|| panic_missing_res::<T>());

		unsafe { ResMut::new(cell, world.tick.get(), change_tick) }
	}
}
