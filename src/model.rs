use serde::Serialize;
use uuid::Uuid;

/// A model representing a single user.
///
/// Use this when fetching from the database and returning to the client.
/// The `email` and `password` fields are not serialized to the client.
#[derive(Debug, Serialize)]
pub struct User {
	pub id: Uuid,
	#[serde(skip_serializing)]
	pub email: String,
	/// argon2 and salted with `id`
	#[serde(skip_serializing)]
	pub password: Vec<u8>,
	pub username: String,
	pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A model representing a single post.
///
/// Use this when fetching from the database and returning to the client.
#[derive(Debug, Serialize)]
pub struct Post {
	pub id: Uuid,
	pub user_id: Uuid,
	pub title: String,
	pub content: String,
	pub created_at: chrono::DateTime<chrono::Utc>,
}
