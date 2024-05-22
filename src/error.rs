#![allow(clippy::module_name_repetitions)]

use std::borrow::Cow;

use aide::OperationOutput;
use axum::{
	body::Body,
	extract::rejection,
	http::{HeaderMap, Response, StatusCode},
	response::IntoResponse,
	Json,
};
use axum_jsonschema::JsonSchemaRejection;
use schemars::JsonSchema;
use serde::Serialize;
use tower_governor::GovernorError;

pub use std::collections::HashMap as Map;

pub trait ErrorShape: Sized {
	fn errors(&self) -> Vec<Message<'_>>;
	fn status(&self) -> StatusCode;
	fn headers(&self) -> Option<HeaderMap> {
		None
	}

	fn into_response(self) -> Response<Body> {
		let mut response = Json(Shape {
			success: false,
			errors: self.errors(),
		})
		.into_response();

		*response.status_mut() = self.status();

		if let Some(headers) = self.headers() {
			*response.headers_mut() = headers;
		}

		response
	}
}

/// Error type for the application.
///
/// The Display trait is not sent to the client, so it can show
/// sensitive information.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum AppError {
	#[error("validation error: {0}")]
	Validation(#[from] validator::ValidationErrors),
	#[error("json schema error")]
	Json(axum_jsonschema::JsonSchemaRejection),
	#[error("query error: {0}")]
	Query(#[from] rejection::QueryRejection),
	#[error("database error: {0}")]
	Database(#[from] sqlx::Error),
	#[error("governor error: {0}")]
	Governor(#[from] tower_governor::GovernorError),
}

impl From<axum_jsonschema::JsonSchemaRejection> for AppError {
	fn from(error: axum_jsonschema::JsonSchemaRejection) -> Self {
		Self::Json(error)
	}
}

macro_rules! impl_from_error {
	($($error:ty),*) => {
		$(
			impl<E> From<$error> for RouteError<E> {
				fn from(error: $error) -> Self {
					Self::App(error.into())
				}
			}
		)*
	};
}

impl_from_error!(
	validator::ValidationErrors,
	axum_jsonschema::JsonSchemaRejection,
	rejection::QueryRejection,
	sqlx::Error,
	tower_governor::GovernorError
);

impl IntoResponse for AppError {
	fn into_response(self) -> Response<Body> {
		ErrorShape::into_response(self)
	}
}

impl ErrorShape for AppError {
	fn status(&self) -> StatusCode {
		match self {
			Self::Validation(..) | Self::Json(..) | Self::Query(..) => StatusCode::BAD_REQUEST,
			Self::Database(..) => StatusCode::INTERNAL_SERVER_ERROR,
			Self::Governor(error) => match error {
				GovernorError::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,
				GovernorError::UnableToExtractKey => StatusCode::INTERNAL_SERVER_ERROR,
				GovernorError::Other { code, .. } => *code,
			},
		}
	}

	fn headers(&self) -> Option<HeaderMap> {
		match self {
			Self::Governor(
				GovernorError::TooManyRequests { headers, .. }
				| GovernorError::Other { headers, .. },
			) => (*headers).clone(),
			_ => None,
		}
	}

	fn errors(&self) -> Vec<Message<'_>> {
		match self {
			Self::Validation(errors) => errors
				.field_errors()
				.into_iter()
				.flat_map(move |(field, errors)| {
					errors.iter().map(move |error| Message {
						content: error.code.as_ref().into(),
						field: Some(field.into()),
						details: Some(Cow::Borrowed(&error.params)),
					})
				})
				.collect(),
			Self::Json(error) => match error {
				JsonSchemaRejection::Json(error) => vec![Message::new(error.to_string().into())],
				JsonSchemaRejection::Serde(error) => vec![Message::new(error.to_string().into())],
				JsonSchemaRejection::Schema(error) => error
					.iter()
					.map(|v| Message {
						content: v.error_description().to_string().into(),
						field: Some(v.instance_location().to_string().into()),
						details: None,
					})
					.collect(),
			},
			Self::Query(error) => vec![Message::new(error.to_string().into())],
			Self::Governor(error) => match error {
				GovernorError::TooManyRequests { .. } => vec![Message {
					content: "too many requests".into(),
					field: None,
					details: None,
				}],
				GovernorError::UnableToExtractKey => {
					vec![Message::new("unable to extract key".into())]
				}
				GovernorError::Other { msg, .. } => msg.as_ref().map_or_else(Vec::new, |msg| {
					vec![Message {
						content: Cow::Borrowed(msg),
						field: None,
						details: None,
					}]
				}),
			},
			Self::Database(..) => Vec::new(),
		}
	}
}

pub enum RouteError<E> {
	App(AppError),
	Route(E),
}

impl<E> From<E> for RouteError<E>
where
	E: ErrorShape,
{
	fn from(error: E) -> Self {
		Self::Route(error)
	}
}

impl<E> OperationOutput for RouteError<E> {
	type Inner = Shape<'static>;

	fn operation_response(
		ctx: &mut aide::gen::GenContext,
		operation: &mut aide::openapi::Operation,
	) -> Option<aide::openapi::Response> {
		Json::<Self::Inner>::operation_response(ctx, operation)
	}
}

impl<E> IntoResponse for RouteError<E>
where
	E: ErrorShape,
{
	fn into_response(self) -> Response<Body> {
		match self {
			Self::App(error) => ErrorShape::into_response(error),
			Self::Route(error) => error.into_response(),
		}
	}
}

impl<E> ErrorShape for RouteError<E>
where
	E: ErrorShape,
{
	fn errors(&self) -> Vec<Message<'_>> {
		match self {
			Self::App(error) => error.errors(),
			Self::Route(error) => error.errors(),
		}
	}

	fn status(&self) -> StatusCode {
		match self {
			Self::App(error) => error.status(),
			Self::Route(error) => error.status(),
		}
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
	pub details: Option<Cow<'e, Map<Cow<'e, str>, serde_json::Value>>>,
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
