use serde::{Deserialize, Serialize};

use super::interface::BlockBuilderInfo;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockBuilderInfoResponse {
    pub block_builder_info: Vec<BlockBuilderInfo>,
}
