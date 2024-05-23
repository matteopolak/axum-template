use macros::model;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

fn validate_username(username: &str) -> Result<(), ValidationError> {
	if username.chars().any(|c| !c.is_alphanumeric()) {
		return Err(ValidationError::new("username must be alphanumeric"));
	}

	Ok(())
}

/// A single user.
#[model]
#[derive(Debug, Deserialize, Serialize, JsonSchema, Validate)]
pub struct User {
	/// The unique identifier of the user.
	#[serde(skip_deserializing)]
	pub id: Uuid,
	/// The user's primary email address, used for logging in and password resets.
	#[serde(skip_serializing)]
	#[validate(email)]
	#[allow(dead_code)]
	pub email: String,
	/// The hashed password.
	#[serde(skip)]
	pub password: Vec<u8>,
	/// The username that is displayed to the public.
	#[validate(length(min = 3, max = 16), custom(function = "validate_username"))]
	pub username: String,
	/// The creation time of the user.
	#[serde(skip_deserializing)]
	pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Validate, JsonSchema)]
pub struct Session {
	/// The session id.
	#[serde(skip_deserializing, rename = "session_id")]
	pub id: Uuid,
	/// The user that owns the session.
	#[serde(skip)]
	#[allow(dead_code)]
	pub user_id: Uuid,
	/// The creation time of the session.
	#[serde(skip_deserializing)]
	pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, Validate, JsonSchema)]
pub struct LoginInput {
	#[validate(email)]
	pub email: String,
	#[validate(length(min = 8, max = 128))]
	pub password: String,
}

#[derive(Deserialize, Validate, JsonSchema)]
pub struct RegisterInput {
	#[validate(email)]
	pub email: String,
	#[validate(length(min = 8, max = 128))]
	pub password: String,
	/// The username that is displayed to the public.
	#[validate(length(min = 3, max = 16), custom(function = "validate_username"))]
	pub username: String,
}
