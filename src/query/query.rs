pub use self::impls::*;

use crate::data::Entity;
use crate::query::iter::*;
use crate::query::ComponentView;
use crate::world::get_subgroup_len;

pub unsafe trait Query<'a> {
    type Item: 'a;
    type Iterator: Iterator<Item = Self::Item> + 'a;

    fn get(self, entity: Entity) -> Option<Self::Item>;

    fn iter(self) -> Self::Iterator;

    fn is_grouped(&self) -> bool;
}

macro_rules! impl_query {
    ($iter:ident, $(($comp:ident, $idx:tt)),+) => {
        unsafe impl<'a, $($comp),+> Query<'a> for ($($comp,)+)
        where
            $($comp: ComponentView<'a> + 'a,)+
        {
            type Item = ($($comp::Item,)+);
            type Iterator = $iter<'a, $($comp),+>;

            fn get(self, entity: Entity) -> Option<Self::Item> {
                Some((
                    $(self.$idx.get(entity)?,)+
                ))
            }

            fn iter(self) -> Self::Iterator {
                $iter::new($(self.$idx),+)
            }

            fn is_grouped(&self) -> bool {
                (|| -> Option<_> {
                    get_subgroup_len(&[
                        $(self.$idx.subgroup_info()?),+
                    ])
                })()
                .is_some()
            }
        }
    };
}

#[rustfmt::skip]
mod impls {
    use super::*;

    impl_query!(Iter2,  (A, 0), (B, 1));
    impl_query!(Iter3,  (A, 0), (B, 1), (C, 2));
    impl_query!(Iter4,  (A, 0), (B, 1), (C, 2), (D, 3));
    impl_query!(Iter5,  (A, 0), (B, 1), (C, 2), (D, 3), (E, 4));
    impl_query!(Iter6,  (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5));
    impl_query!(Iter7,  (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6));
    impl_query!(Iter8,  (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7));
    impl_query!(Iter9,  (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8));
    impl_query!(Iter10, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9));
    impl_query!(Iter11, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10));
    impl_query!(Iter12, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11));
    impl_query!(Iter13, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11), (M, 12));
    impl_query!(Iter14, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11), (M, 12), (N, 13));
    impl_query!(Iter15, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11), (M, 12), (N, 13), (O, 14));
    impl_query!(Iter16, (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7), (I, 8), (J, 9), (K, 10), (L, 11), (M, 12), (N, 13), (O, 14), (P, 15));
}
