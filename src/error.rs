use std::{borrow::Cow, collections::HashMap};

use aide::OperationOutput;
use axum::{
	body::Body,
	extract::rejection,
	http::{Response, StatusCode},
	response::IntoResponse,
	Json,
};
use axum_jsonschema::JsonSchemaRejection;
use schemars::JsonSchema;
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
	#[error("json schema error")]
	Json(axum_jsonschema::JsonSchemaRejection),
	#[error("query error: {0}")]
	Query(#[from] rejection::QueryRejection),
	#[error("auth error: {0}")]
	Auth(#[from] auth::Error),
	#[error("post error: {0}")]
	Post(#[from] posts::Error),
	#[error("database error: {0}")]
	Database(#[from] sqlx::Error),
}

impl From<axum_jsonschema::JsonSchemaRejection> for Error {
	fn from(error: axum_jsonschema::JsonSchemaRejection) -> Self {
		Self::Json(error)
	}
}

/// Error shape returned to the client.
///
/// This is used for all application errors, such as database
/// connectivity, authentication, and schema validation.
#[derive(Debug, Serialize, JsonSchema)]
pub struct Shape<'e> {
	pub success: bool,
	pub errors: Vec<Message<'e>>,
}

/// Single error message shape.
///
/// Represents a single error message, often accompanied by
/// additional context (e.g. when a validation error displays
/// requirements).
#[derive(Debug, Serialize, JsonSchema)]
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

impl OperationOutput for Error {
	type Inner = Shape<'static>;

	fn operation_response(
		ctx: &mut aide::gen::GenContext,
		operation: &mut aide::openapi::Operation,
	) -> Option<aide::openapi::Response> {
		Json::<Self::Inner>::operation_response(ctx, operation)
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
			Error::Json(error) => match error {
				JsonSchemaRejection::Json(error) => (
					StatusCode::BAD_REQUEST,
					vec![Message::new(error.to_string().into())],
				),
				JsonSchemaRejection::Serde(error) => (
					StatusCode::BAD_REQUEST,
					vec![Message::new(error.to_string().into())],
				),
				JsonSchemaRejection::Schema(error) => (
					StatusCode::BAD_REQUEST,
					error
						.into_iter()
						.map(|v| Message {
							content: v.error_description().to_string().into(),
							field: Some(v.instance_location().to_string().into()),
							details: None,
						})
						.collect(),
				),
			},
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
