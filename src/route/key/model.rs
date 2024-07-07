pub use crate::route::model::{IdInput, Paginate};

use schemars::JsonSchema;
use serde::Serialize;
use uuid::Uuid;

/// A single API key, owned by a user and used to perform automated
/// actions on their behalf.
#[derive(Debug, Serialize, JsonSchema)]
pub struct Key {
	/// The API key.
	#[serde(skip_deserializing)]
	#[serde(rename = "key")]
	pub id: Uuid,
	/// The user that owns the key.
	#[serde(skip)]
	#[allow(dead_code)]
	pub user_id: Uuid,
	/// The creation time of the key.
	#[serde(skip_deserializing)]
	pub created_at: chrono::DateTime<chrono::Utc>,
}
