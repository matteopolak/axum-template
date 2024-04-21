use std::{borrow::Cow, collections::HashMap};

use axum::{
	body::Body,
	extract::rejection,
	http::{Response, StatusCode},
	response::IntoResponse,
	Json,
};
use serde::Serialize;

use crate::route::{auth::AuthError, posts::PostError};

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
	#[error("query error: {0}")]
	Query(#[from] rejection::QueryRejection),
	#[error("auth error: {0}")]
	Auth(#[from] AuthError),
	#[error("post error: {0}")]
	Post(#[from] PostError),
	#[error("database error: {0}")]
	Database(#[from] sqlx::Error),
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse<'e> {
	pub success: bool,
	pub errors: Vec<ErrorMessage<'e>>,
}

#[derive(Debug, Serialize)]
pub struct ErrorMessage<'e> {
	pub message: Cow<'e, str>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub field: Option<Cow<'e, str>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub details: Option<&'e HashMap<Cow<'e, str>, serde_json::Value>>,
}

impl<'e> ErrorMessage<'e> {
	pub fn new(message: Cow<'e, str>) -> Self {
		Self {
			message,
			field: None,
			details: None,
		}
	}
}

impl IntoResponse for Error {
	fn into_response(self) -> Response<Body> {
		tracing::error!("error: {:?}", self);

		let (status, errors) = match self {
			Error::Validation(errors) => {
				return (
					StatusCode::BAD_REQUEST,
					Json(ErrorResponse {
						success: false,
						errors: errors
							.field_errors()
							.into_iter()
							.flat_map(move |(field, errors)| {
								errors.iter().map(move |error| ErrorMessage {
									message: error.code.as_ref().into(),
									field: Some(field.into()),
									details: Some(&error.params),
								})
							})
							.collect(),
					}),
				)
					.into_response()
			}
			Error::Auth(error) => (
				StatusCode::UNAUTHORIZED,
				vec![ErrorMessage::new(error.to_string().into())],
			),
			Error::Json(error) => (
				StatusCode::BAD_REQUEST,
				vec![ErrorMessage::new(error.to_string().into())],
			),
			Error::Post(error) => (
				StatusCode::NOT_FOUND,
				vec![ErrorMessage::new(error.to_string().into())],
			),
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
