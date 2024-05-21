use schemars::JsonSchema;
use serde::Serialize;
use uuid::Uuid;

/// A single user.
#[derive(Debug, Serialize, JsonSchema)]
pub struct User {
	/// The unique identifier of the user.
	#[serde(skip_deserializing)]
	pub id: Uuid,
	/// The user's primary email address, used for logging in and password resets.
	#[serde(skip_serializing)]
	#[allow(dead_code)]
	pub email: String,
	/// The hashed password.
	#[serde(skip_serializing)]
	pub password: Vec<u8>,
	/// The username that is displayed to the public.
	pub username: String,
	/// The creation time of the user.
	#[serde(skip_deserializing)]
	pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A single post, created by a user.
#[derive(Debug, Serialize, JsonSchema)]
pub struct Post {
	/// The unique identifier of the post.
	#[serde(skip_deserializing)]
	pub id: Uuid,
	/// The user that created the post.
	#[serde(skip_deserializing)]
	pub user_id: Uuid,
	/// The title of the post.
	pub title: String,
	/// The content of the post in Markdown format.
	pub content: String,
	/// The creation time of the post.
	#[serde(skip_deserializing)]
	pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A single API key, owned by a user and used to perform automated
/// actions on their behalf.
#[derive(Debug, Serialize, JsonSchema)]
pub struct Key {
	/// The API key.
	#[serde(skip_deserializing)]
	pub id: Uuid,
	/// The user that owns the key.
	#[serde(skip_deserializing)]
	#[allow(dead_code)]
	pub user_id: Uuid,
	/// The creation time of the key.
	#[serde(skip_deserializing)]
	pub created_at: chrono::DateTime<chrono::Utc>,
}
