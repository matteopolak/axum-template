use std::{borrow::Cow, collections::HashMap};

use axum::{
	body::Body,
	extract::rejection,
	http::{Response, StatusCode},
	response::IntoResponse,
	Json,
};
use serde::Serialize;

use crate::route::{auth, posts};

/// Error type for the application.
///
/// The Display trait is not sent to the client, so it can show
/// sensitive information.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("validation error: {0}")]
	Validation(#[from] validator::ValidationErrors),
	#[error("json error: {0}")]
	Json(#[from] rejection::JsonRejection),
	#[error("query error: {0}")]
	Query(#[from] rejection::QueryRejection),
	#[error("auth error: {0}")]
	Auth(#[from] auth::Error),
	#[error("post error: {0}")]
	Post(#[from] posts::Error),
	#[error("database error: {0}")]
	Database(#[from] sqlx::Error),
}

/// Error shape returned to the client.
///
/// This is used for all application errors, such as database
/// connectivity, authentication, and schema validation.
#[derive(Debug, Serialize)]
pub struct Shape<'e> {
	pub success: bool,
	pub errors: Vec<Message<'e>>,
}

/// Single error message shape.
///
/// Represents a single error message, often accompanied by
/// additional context (e.g. when a validation error displays
/// requirements).
#[derive(Debug, Serialize)]
pub struct Message<'e> {
	pub content: Cow<'e, str>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub field: Option<Cow<'e, str>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub details: Option<&'e HashMap<Cow<'e, str>, serde_json::Value>>,
}

impl<'e> Message<'e> {
	pub fn new(content: Cow<'e, str>) -> Self {
		Self {
			content,
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
					Json(Shape {
						success: false,
						errors: errors
							.field_errors()
							.into_iter()
							.flat_map(move |(field, errors)| {
								errors.iter().map(move |error| Message {
									content: error.code.as_ref().into(),
									field: Some(field.into()),
									details: Some(&error.params),
								})
							})
							.collect(),
					}),
				)
					.into_response()
			}
			Error::Auth(error) => (error.status(), vec![Message::new(error.to_string().into())]),
			Error::Json(error) => (
				StatusCode::BAD_REQUEST,
				vec![Message::new(error.to_string().into())],
			),
			Error::Query(error) => (
				StatusCode::BAD_REQUEST,
				vec![Message::new(error.to_string().into())],
			),
			Error::Post(error) => (error.status(), vec![Message::new(error.to_string().into())]),
			_ => (StatusCode::INTERNAL_SERVER_ERROR, Vec::new()),
		};

		(
			status,
			Json(Shape {
				errors,
				success: false,
			}),
		)
			.into_response()
	}
}
