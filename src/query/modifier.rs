use crate::components::QueryGroupInfo;
use crate::query::{GetComponentUnfiltered, GetImmutableComponentUnfiltered, Passthrough};
use crate::storage::{Entity, EntitySparseArray};
use crate::utils::impl_generic_tuple_1_16;

/// Trait used to include/exclude components from a `Query`.
pub unsafe trait QueryModifier<'a> {
    /// Whether or not the `Query` matches all inputs. Used internally by queries for optimization
    /// purposes.
    const IS_PASSTHROUGH: bool = false;

    /// `EntitySparseArray`s returned when splitting the views.
    type Sparse: 'a;

    /// Returns `true` if all views include `entity`.
    fn includes(&self, entity: Entity) -> bool;

    /// Returns `true` if all views exclude `entity`.
    fn excludes(&self, entity: Entity) -> bool;

    /// Includes the views' `QueryGroupInfo` in the provided `info`, if possible.
    fn include_group_info(&self, info: QueryGroupInfo<'a>) -> Option<QueryGroupInfo<'a>>;

    /// Includes the views' `QueryGroupInfo` from the provided `info`, if possible.
    fn exclude_group_info(&self, info: QueryGroupInfo<'a>) -> Option<QueryGroupInfo<'a>>;

    /// Splits the modifier into the shortest `Entity` slice and the views' `EntitySparseArray`s.
    fn split(self) -> (Option<&'a [Entity]>, Self::Sparse);

    /// Returns `true` if all views include `entity`.
    fn includes_sparse(sparse: &Self::Sparse, entity: Entity) -> bool;

    /// Returns `true` if all views exclude `entity`.
    fn excludes_sparse(sparse: &Self::Sparse, entity: Entity) -> bool;
}

unsafe impl<'a> QueryModifier<'a> for Passthrough {
    const IS_PASSTHROUGH: bool = true;

    type Sparse = ();

    #[inline(always)]
    fn includes(&self, _entity: Entity) -> bool {
        true
    }

    #[inline(always)]
    fn excludes(&self, _entity: Entity) -> bool {
        true
    }

    #[inline(always)]
    fn include_group_info(&self, info: QueryGroupInfo<'a>) -> Option<QueryGroupInfo<'a>> {
        Some(info)
    }

    #[inline(always)]
    fn exclude_group_info(&self, info: QueryGroupInfo<'a>) -> Option<QueryGroupInfo<'a>> {
        Some(info)
    }

    #[inline(always)]
    fn split(self) -> (Option<&'a [Entity]>, Self::Sparse) {
        (None, ())
    }

    #[inline(always)]
    fn includes_sparse(_sparse: &Self::Sparse, _entity: Entity) -> bool {
        true
    }

    #[inline(always)]
    fn excludes_sparse(_sparse: &Self::Sparse, _entity: Entity) -> bool {
        true
    }
}

unsafe impl<'a, G> QueryModifier<'a> for G
where
    G: GetImmutableComponentUnfiltered<'a>,
{
    type Sparse = &'a EntitySparseArray;

    fn includes(&self, entity: Entity) -> bool {
        GetComponentUnfiltered::get_index(self, entity).is_some()
    }

    fn excludes(&self, entity: Entity) -> bool {
        GetComponentUnfiltered::get_index(self, entity).is_none()
    }

    fn include_group_info(&self, info: QueryGroupInfo<'a>) -> Option<QueryGroupInfo<'a>> {
        info.include(GetComponentUnfiltered::group_info(self)?)
    }

    fn exclude_group_info(&self, info: QueryGroupInfo<'a>) -> Option<QueryGroupInfo<'a>> {
        info.exclude(GetComponentUnfiltered::group_info(self)?)
    }

    fn split(self) -> (Option<&'a [Entity]>, Self::Sparse) {
        let (entities, sparse, _) = GetComponentUnfiltered::split(self);
        (Some(entities), sparse)
    }

    fn includes_sparse(sparse: &Self::Sparse, entity: Entity) -> bool {
        sparse.contains(entity)
    }

    fn excludes_sparse(sparse: &Self::Sparse, entity: Entity) -> bool {
        !sparse.contains(entity)
    }
}

macro_rules! entity_sparse_array {
    ($elem:ident) => {
        &'a EntitySparseArray
    };
}

macro_rules! impl_query_modifier {
    ($(($elem:ident, $idx:tt)),+) => {
        unsafe impl<'a, $($elem),+> QueryModifier<'a> for ($($elem,)+)
        where
            $($elem: GetImmutableComponentUnfiltered<'a>,)+
        {
            type Sparse = ($(entity_sparse_array!($elem),)+);

            fn includes(&self, entity: Entity) -> bool {
                $(self.$idx.get_index(entity).is_some())&&+
            }

            fn excludes(&self, entity: Entity) -> bool {
                $(self.$idx.get_index(entity).is_none())&&+
            }

            #[allow(clippy::needless_question_mark)]
            fn include_group_info(&self, info: QueryGroupInfo<'a>) -> Option<QueryGroupInfo<'a>> {
                Some(info $(.include(self.$idx.group_info()?)?)+)
            }

            #[allow(clippy::needless_question_mark)]
            fn exclude_group_info(&self, info: QueryGroupInfo<'a>) -> Option<QueryGroupInfo<'a>> {
                Some(info $(.exclude(self.$idx.group_info()?)?)+)
            }

            fn split(self) -> (Option<&'a [Entity]>, Self::Sparse) {
                split_modifier!($((self.$idx, $idx)),+)
            }

            fn includes_sparse(sparse: &Self::Sparse, entity: Entity) -> bool {
                $(sparse.$idx.contains(entity))&&+
            }

            fn excludes_sparse(sparse: &Self::Sparse, entity: Entity) -> bool {
                $(!sparse.$idx.contains(entity))&&+
            }
        }
    };
}

impl_generic_tuple_1_16!(impl_query_modifier);
