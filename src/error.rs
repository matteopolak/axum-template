use axum::{
	body::Body,
	extract::rejection,
	http::{Response, StatusCode},
	response::IntoResponse,
	Json,
};
use serde::Serialize;

use crate::route::auth::AuthError;

/// Error type for the application.
///
/// The Display trait is not sent to the client, so it can show
/// sensitive information.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("validation error: {0}")]
	Validation(#[from] validator::ValidationErrors),
	#[error("json error: {0}")]
	Json(#[from] rejection::JsonRejection),
	#[error("auth error: {0}")]
	Auth(#[from] AuthError),
	#[error("database error: {0}")]
	Database(#[from] sqlx::Error),
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
	pub success: bool,
	pub errors: Vec<String>,
}

impl IntoResponse for Error {
	fn into_response(self) -> Response<Body> {
		match self {
			Error::Validation(errors) => (
				StatusCode::BAD_REQUEST,
				Json(ErrorResponse {
					errors: errors
						.field_errors()
						.into_iter()
						.map(move |(field, errors)| {
							errors
								.into_iter()
								.map(move |error| format!("{}: {}", field, error))
						})
						.flatten()
						.collect(),
					success: false,
				}),
			)
				.into_response(),
			Error::Auth(error) => (
				StatusCode::UNAUTHORIZED,
				Json(ErrorResponse {
					errors: vec![error.to_string()],
					success: false,
				}),
			)
				.into_response(),
			Error::Json(error) => (
				StatusCode::BAD_REQUEST,
				Json(ErrorResponse {
					errors: vec![error.to_string()],
					success: false,
				}),
			)
				.into_response(),
			_ => (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(ErrorResponse {
					errors: Vec::new(),
					success: false,
				}),
			)
				.into_response(),
		}
	}
}
