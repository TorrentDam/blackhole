use k8s_openapi::api::core::v1::ResourceRequirements;
use kube::{
    CustomResource,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[cfg_attr(test, derive(Default))]
#[kube(kind = "Blackhole", group = "torrentdam.org", version = "v1", namespaced)]
#[kube(shortname = "blackhole")]
pub struct BlackholeSpec {
    pub resources: Option<ResourceRequirements>,
}