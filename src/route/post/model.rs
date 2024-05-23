pub use crate::route::model::PaginateInput;

use macros::model;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// A single post, created by a user.
#[model]
#[derive(Debug, Deserialize, Serialize, JsonSchema, Validate)]
pub struct Post {
	/// The unique identifier of the post.
	#[serde(skip_deserializing)]
	pub id: Uuid,
	/// The user that created the post.
	#[serde(skip_deserializing)]
	pub user_id: Uuid,
	/// The title of the post.
	#[validate(length(min = 3, max = 128))]
	pub title: String,
	/// The content of the post in Markdown format.
	pub content: String,
	/// The creation time of the post.
	#[serde(skip_deserializing)]
	pub created_at: chrono::DateTime<chrono::Utc>,
}
