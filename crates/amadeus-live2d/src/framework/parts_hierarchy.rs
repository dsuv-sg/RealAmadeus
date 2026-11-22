//! Parts hierarchy — `CubismModel.setupPartsHierarchy()` / `getPartChildDrawObjects()` (spec §7.8).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).

/// `NoParentIndex`.
pub const NO_PARENT_INDEX: i32 = -1;
/// `NoOffscreenIndex`.
pub const NO_OFFSCREEN_INDEX: i32 = -1;

/// `CubismModelObjectType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CubismModelObjectType {
    Drawable = 0,
    Parts = 1,
}

/// `CubismModelObjectInfo { objectIndex, objectType }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CubismModelObjectInfo {
    pub object_index: usize,
    pub object_type: CubismModelObjectType,
}

/// `PartChildDrawObjects { drawableIndices, offscreenIndices }`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartChildDrawObjects {
    pub drawable_indices: Vec<usize>,
    pub offscreen_indices: Vec<usize>,
}

/// `CubismModelPartInfo { objects, childDrawObjects }`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CubismModelPartInfo {
    pub objects: Vec<CubismModelObjectInfo>,
    pub child_draw_objects: PartChildDrawObjects,
}

impl CubismModelPartInfo {
    /// `getChildObjectCount()`.
    pub fn child_object_count(&self) -> usize {
        self.objects.len()
    }
}

/// `setupPartsHierarchy()`: child parts and drawables per part (ascending index order), then the
/// recursively flattened drawable/offscreen lists of every part.
pub fn setup_parts_hierarchy(
    part_parent_indices: &[i32],
    drawable_parent_part_indices: &[i32],
    part_offscreen_indices: &[i32],
) -> Vec<CubismModelPartInfo> {
    let part_count = part_parent_indices.len();
    let mut hierarchy = vec![CubismModelPartInfo::default(); part_count];
    for (i, &parent) in part_parent_indices.iter().enumerate() {
        if parent == NO_PARENT_INDEX {
            continue;
        }
        if let Some(info) = usize::try_from(parent)
            .ok()
            .and_then(|p| hierarchy.get_mut(p))
        {
            info.objects.push(CubismModelObjectInfo {
                object_index: i,
                object_type: CubismModelObjectType::Parts,
            });
        }
    }
    for (i, &parent) in drawable_parent_part_indices.iter().enumerate() {
        if parent == NO_PARENT_INDEX {
            continue;
        }
        if let Some(info) = usize::try_from(parent)
            .ok()
            .and_then(|p| hierarchy.get_mut(p))
        {
            info.objects.push(CubismModelObjectInfo {
                object_index: i,
                object_type: CubismModelObjectType::Drawable,
            });
        }
    }
    for i in 0..part_count {
        part_child_draw_objects(&mut hierarchy, part_offscreen_indices, i, 0);
    }
    hierarchy
}

/// `getPartChildDrawObjects(partInfoIndex)` (memoised in `child_draw_objects`). `depth` guards
/// against cyclic parent data (the Core never produces cycles).
fn part_child_draw_objects(
    hierarchy: &mut [CubismModelPartInfo],
    part_offscreen_indices: &[i32],
    part: usize,
    depth: usize,
) {
    let Some(info) = hierarchy.get(part) else {
        return;
    };
    if info.child_object_count() < 1 {
        return;
    }
    let child = &info.child_draw_objects;
    if !child.drawable_indices.is_empty()
        || !child.offscreen_indices.is_empty()
        || depth > hierarchy.len()
    {
        return;
    }
    let objects = info.objects.clone();
    let mut result = PartChildDrawObjects::default();
    for obj in objects {
        match obj.object_type {
            CubismModelObjectType::Parts => {
                part_child_draw_objects(
                    hierarchy,
                    part_offscreen_indices,
                    obj.object_index,
                    depth + 1,
                );
                if let Some(sub) = hierarchy.get(obj.object_index) {
                    result
                        .drawable_indices
                        .extend_from_slice(&sub.child_draw_objects.drawable_indices);
                    result
                        .offscreen_indices
                        .extend_from_slice(&sub.child_draw_objects.offscreen_indices);
                }
                let offscreen = part_offscreen_indices
                    .get(obj.object_index)
                    .copied()
                    .unwrap_or(NO_OFFSCREEN_INDEX);
                if offscreen != NO_OFFSCREEN_INDEX {
                    if let Ok(o) = usize::try_from(offscreen) {
                        result.offscreen_indices.push(o);
                    }
                }
            }
            CubismModelObjectType::Drawable => result.drawable_indices.push(obj.object_index),
        }
    }
    hierarchy[part].child_draw_objects = result;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kurisu_hierarchy_matches_golden() {
        // golden/model_info.json
        let parents = [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 4, 5];
        let drawables = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 9, 10, 1, 1, 2, 6, 11, 12, 14, 14, 14, 14, 14, 5, 5,
            4, 4, 13, 13, 13, 13, 13, 3, 3, 3, 3, 3, 3, 3, 8, 8, 0, 4, 4, 13, 14, 5, 5,
        ];
        let h = setup_parts_hierarchy(&parents, &drawables, &[-1; 15]);
        assert_eq!(
            h[4].child_draw_objects.drawable_indices,
            vec![28, 29, 30, 31, 32, 45, 26, 27, 43, 44]
        );
        assert_eq!(
            h[5].child_draw_objects.drawable_indices,
            vec![19, 20, 21, 22, 23, 46, 24, 25, 47, 48]
        );
        assert_eq!(
            h[4].objects.first(),
            Some(&CubismModelObjectInfo {
                object_index: 13,
                object_type: CubismModelObjectType::Parts
            })
        );
        assert_eq!(
            h[0].child_draw_objects.drawable_indices,
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 42]
        );
    }
}
