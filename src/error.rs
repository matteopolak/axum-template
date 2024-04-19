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
		let (status, errors) = match self {
			Error::Validation(errors) => (
				StatusCode::BAD_REQUEST,
				errors
					.field_errors()
					.into_iter()
					.flat_map(move |(field, errors)| {
						errors
							.iter()
							.map(move |error| format!("{}: {}", field, error))
					})
					.collect(),
			),
			Error::Auth(error) => (StatusCode::UNAUTHORIZED, vec![error.to_string()]),
			Error::Json(error) => (StatusCode::BAD_REQUEST, vec![error.to_string()]),
			_ => (StatusCode::INTERNAL_SERVER_ERROR, Vec::new()),
		};

		(
			status,
			Json(ErrorResponse {
				errors,
				success: false,
			}),
		)
			.into_response()
	}
}
