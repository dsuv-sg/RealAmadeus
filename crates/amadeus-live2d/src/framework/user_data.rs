//! `CubismModelUserData` — userdata3.json, port of `Framework/src/model/cubismmodeluserdata.ts` and
//! `cubismmodeluserdatajson.ts` (spec §9).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).

use super::json::{self, get, index};
use crate::core::model::Live2DError;

/// `ArtMesh` target type name.
pub const ART_MESH: &str = "ArtMesh";

/// `CubismModelUserDataNode { targetType, targetId, value }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CubismModelUserDataNode {
    pub target_type: String,
    pub target_id: String,
    pub value: String,
}

/// `CubismModelUserData`.
#[derive(Debug, Clone, Default)]
pub struct CubismModelUserData {
    user_data_nodes: Vec<CubismModelUserDataNode>,
    art_mesh_user_data_node: Vec<CubismModelUserDataNode>,
    total_user_data_size: usize,
}

impl CubismModelUserData {
    /// `CubismModelUserData.create(buffer, size)`: `Meta.UserDataCount` nodes from `UserData[i]`.
    pub fn create(bytes: &[u8]) -> Result<Self, Live2DError> {
        let root = json::parse_json(bytes, "userdata3.json")?;
        let r = Some(&root);
        let meta = get(r, "Meta");
        let node_count = json::to_count(get(meta, "UserDataCount"));
        let user_data = get(r, "UserData");
        let mut ret = Self {
            total_user_data_size: json::to_count(get(meta, "TotalUserDataSize")),
            ..Self::default()
        };
        for i in 0..node_count {
            let entry = index(user_data, i);
            let node = CubismModelUserDataNode {
                target_id: json::raw_string_or_empty(get(entry, "Id")),
                target_type: json::raw_string_or_empty(get(entry, "Target")),
                value: json::raw_string_or_empty(get(entry, "Value")),
            };
            if node.target_type == ART_MESH {
                ret.art_mesh_user_data_node.push(node.clone());
            }
            ret.user_data_nodes.push(node);
        }
        Ok(ret)
    }

    /// `getArtMeshUserDatas()`.
    pub fn art_mesh_user_datas(&self) -> &[CubismModelUserDataNode] {
        &self.art_mesh_user_data_node
    }

    /// Every node in file order.
    pub fn user_data_nodes(&self) -> &[CubismModelUserDataNode] {
        &self.user_data_nodes
    }

    /// `Meta.TotalUserDataSize` (unused by the Framework).
    pub fn total_user_data_size(&self) -> usize {
        self.total_user_data_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nodes() {
        let src = br#"{"Version":3,"Meta":{"UserDataCount":2,"TotalUserDataSize":10},
            "UserData":[{"Target":"ArtMesh","Id":"ArtMesh1","Value":"hello"},{"Target":"Part","Id":"P","Value":"x"}]}"#;
        let u = CubismModelUserData::create(src).unwrap_or_default();
        assert_eq!(u.user_data_nodes().len(), 2);
        assert_eq!(u.art_mesh_user_datas().len(), 1);
        assert_eq!(u.art_mesh_user_datas()[0].value, "hello");
        assert_eq!(u.total_user_data_size(), 10);
    }
}
