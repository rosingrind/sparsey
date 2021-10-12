use crate::components::Group;
use crate::group::{QueryMask, StorageMask};
use std::ops::Range;
use std::ptr;

/// Tracks the group to which a component storage belongs.
#[derive(Clone, Copy)]
pub struct GroupInfo<'a> {
    group_family: &'a [Group],
    group_offset: usize,
    storage_mask: StorageMask,
}

impl<'a> GroupInfo<'a> {
    pub(crate) const fn new(
        group_family: &'a [Group],
        group_offset: usize,
        storage_mask: StorageMask,
    ) -> Self {
        Self {
            group_family,
            group_offset,
            storage_mask,
        }
    }
}

/// Tracks the group to which a multiple component storages belong.
#[derive(Copy, Clone, Default)]
pub struct CombinedGroupInfo<'a> {
    group_family: Option<&'a [Group]>,
    max_group_offset: usize,
    storage_mask: StorageMask,
}

impl<'a> CombinedGroupInfo<'a> {
    pub(crate) fn combine(self, group_info: GroupInfo<'a>) -> Option<Self> {
        match self.group_family {
            Some(group_family) => {
                ptr::eq(group_family, group_info.group_family).then(|| CombinedGroupInfo {
                    group_family: Some(group_family),
                    max_group_offset: self.max_group_offset.max(group_info.group_offset),
                    storage_mask: self.storage_mask | group_info.storage_mask,
                })
            }
            None => Some(CombinedGroupInfo {
                group_family: Some(group_info.group_family),
                max_group_offset: group_info.group_offset,
                storage_mask: group_info.storage_mask,
            }),
        }
    }
}

fn common_group_family<'a>(group_families: &[Option<&'a [Group]>]) -> Option<&'a [Group]> {
    let mut group_family: Option<&[Group]> = None;

    for &gf in group_families {
        if let Some(gf) = gf {
            match group_family {
                Some(group_family) => {
                    if !ptr::eq(group_family, gf) {
                        return None;
                    }
                }
                None => group_family = Some(gf),
            }
        }
    }

    group_family
}

/// Returns the range of elements the storages have in common if the
/// `CombinedGroupInfo`s form a group.
pub(crate) fn group_range(
    base: CombinedGroupInfo,
    include: CombinedGroupInfo,
    exclude: CombinedGroupInfo,
) -> Option<Range<usize>> {
    let group_family = common_group_family(&[
        base.group_family,
        include.group_family,
        exclude.group_family,
    ])?;

    let max_group_offset = base
        .max_group_offset
        .max(include.max_group_offset)
        .max(exclude.max_group_offset);

    let group_mask = QueryMask::new(
        base.storage_mask | include.storage_mask,
        exclude.storage_mask,
    );
    let group = &group_family[max_group_offset];

    if group_mask == group.include_mask() {
        Some(0..group.len())
    } else if group_mask == group.exclude_mask() {
        let prev_group = &group_family[max_group_offset - 1];
        Some(group.len()..prev_group.len())
    } else {
        None
    }
}
