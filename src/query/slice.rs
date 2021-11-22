use crate::query::{GetComponent, GetComponentSet};
use crate::storage::Entity;

pub trait SliceComponent<'a>
where
    Self: GetComponent<'a>,
{
    fn entities(&self) -> &[Entity];

    fn components(&self) -> &[Self::Component];
}

pub trait SliceComponentSet<'a>
where
    Self: GetComponentSet<'a>,
{
    type Slices;
}
