use crate::layout::ComponentInfo;
use std::any::TypeId;

/// Type of asset borrowed by systems during execution.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum SystemBorrow {
    /// View over all entities.
    Entities,
    /// View over all components of a type.
    Comp(ComponentInfo),
    /// Mutable view over all components of a type.
    CompMut(ComponentInfo),
    /// View over a resource.
    Res(TypeId),
    /// Mutable view over a resource.
    ResMut(TypeId),
}

impl SystemBorrow {
    /// Returns `true` if the parameters prevent the systems from running in parallel.
    pub fn conflicts_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Comp(c1), Self::CompMut(c2)) => c1 == c2,
            (Self::CompMut(c1), Self::Comp(c2)) => c1 == c2,
            (Self::CompMut(c1), Self::CompMut(c2)) => c1 == c2,
            (Self::Res(r1), Self::ResMut(r2)) => r1 == r2,
            (Self::ResMut(r1), Self::Res(r2)) => r1 == r2,
            (Self::ResMut(r1), Self::ResMut(r2)) => r1 == r2,
            _ => false,
        }
    }
}
