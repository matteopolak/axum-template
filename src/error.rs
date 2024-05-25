#![allow(clippy::module_name_repetitions)]

use std::{borrow::Cow, error::Error};

use aide::OperationOutput;
use axum::{
	body::Body,
	extract::rejection::{self, JsonRejection, QueryRejection},
	http::{HeaderMap, Response, StatusCode},
	response::IntoResponse,
	Json,
};
use axum_jsonschema::JsonSchemaRejection;
use schemars::JsonSchema;
use serde::Serialize;
use tower_governor::GovernorError;
use validator::{ValidationErrors, ValidationErrorsKind};

pub use std::collections::HashMap as Map;

pub trait ErrorShape: Sized {
	/// Returns a list of error messages to be sent to the client.
	fn into_errors(self) -> Vec<Message<'static>>;
	/// Returns the HTTP status code associated with the error.
	fn status(&self) -> StatusCode;
	/// Returns additional headers to be sent to the client.
	/// This is useful for rate limiting and other headers.
	fn headers(&self) -> Option<HeaderMap> {
		None
	}

	/// Transforms the error into a response. Unless you need to
	/// customize the response, you should not override this method.
	fn into_response(self) -> Response<Body> {
		let status = self.status();
		let headers = self.headers();
		let mut response = Json(self.into_errors()).into_response();

		*response.status_mut() = status;

		if let Some(headers) = headers {
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

impl<E> From<sqlx::Error> for RouteError<E> {
	fn from(error: sqlx::Error) -> Self {
		Self::App(error.into())
	}
}

impl IntoResponse for AppError {
	fn into_response(self) -> Response<Body> {
		ErrorShape::into_response(self)
	}
}

impl ErrorShape for ValidationErrors {
	fn status(&self) -> StatusCode {
		StatusCode::BAD_REQUEST
	}

	fn into_errors(self) -> Vec<Message<'static>> {
		self.into_errors()
			.into_iter()
			.filter_map(|(k, v)| {
				if let ValidationErrorsKind::Field(errors) = v {
					Some((k, errors))
				} else {
					None
				}
			})
			.flat_map(move |(field, errors)| {
				errors.into_iter().map(move |mut error| Message {
					code: error.code,
					content: error.message,
					details: Some(Cow::Owned({
						error.params.insert("field".into(), field.into());
						error.params
					})),
				})
			})
			.collect()
	}
}

impl ErrorShape for JsonSchemaRejection {
	fn status(&self) -> StatusCode {
		StatusCode::BAD_REQUEST
	}

	fn into_errors(self) -> Vec<Message<'static>> {
		match self {
			Self::Json(error) => match error {
				JsonRejection::JsonDataError(error) => {
					Message::new("json_deserialize_error").content(error.to_string())
				}
				JsonRejection::BytesRejection(error) => {
					Message::new("unacceptable_json_payload").content(error.to_string())
				}
				JsonRejection::JsonSyntaxError(error) => {
					Message::new("json_syntax_error").content(error.to_string())
				}
				JsonRejection::MissingJsonContentType(..) => {
					Message::new("missing_json_content_type").content("Missing JSON content type.")
				}
				_ => Message::new("unknown_json_error").content("Unknown JSON error."),
			}
			.into_vec(),
			Self::Serde(error) => Message::new("json_deserialize_error")
				.detail("field", error.path().to_string())
				.into_vec(),
			Self::Schema(error) => error
				.into_iter()
				// TODO: remove this allocation! https://github.com/Stranger6667/jsonschema-rs/issues/488
				.map(|v| {
					Message::new("json_validation_error").content(v.error_description().to_string())
				})
				.collect(),
		}
	}
}

impl ErrorShape for GovernorError {
	fn status(&self) -> StatusCode {
		match self {
			Self::TooManyRequests { .. } => StatusCode::TOO_MANY_REQUESTS,
			Self::UnableToExtractKey => StatusCode::INTERNAL_SERVER_ERROR,
			Self::Other { code, .. } => *code,
		}
	}

	fn into_errors(self) -> Vec<Message<'static>> {
		match self {
			Self::TooManyRequests { .. } => Message::new("too_many_requests")
				.content("You are sending too many requests.")
				.into_vec(),
			Self::UnableToExtractKey => Message::new("internal_error")
				.content("Unable to extract key.")
				.into_vec(),
			Self::Other { msg, .. } => msg.map_or_else(Vec::new, |msg| {
				Message::new("internal_error").content(msg).into_vec()
			}),
		}
	}
}

impl ErrorShape for QueryRejection {
	fn status(&self) -> StatusCode {
		StatusCode::BAD_REQUEST
	}

	fn into_errors(self) -> Vec<Message<'static>> {
		match self {
			Self::FailedToDeserializeQueryString(error) => {
				Message::new("query_deserialize_error").content(error.to_string())
			}
			_ => Message::new("unknown_query_error").content("Unknown query error."),
		}
		.into_vec()
	}
}

impl ErrorShape for AppError {
	fn status(&self) -> StatusCode {
		match self {
			Self::Validation(errors) => errors.status(),
			Self::Json(error) => error.status(),
			Self::Query(..) => StatusCode::BAD_REQUEST,
			Self::Database(..) => StatusCode::INTERNAL_SERVER_ERROR,
			Self::Governor(error) => error.status(),
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

	fn into_errors(self) -> Vec<Message<'static>> {
		match self {
			Self::Validation(errors) => ErrorShape::into_errors(errors),
			Self::Json(error) => error.into_errors(),
			Self::Query(error) => vec![Message::new(error.to_string())],
			Self::Governor(error) => error.into_errors(),
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
	type Inner = Vec<Message<'static>>;

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
	RouteError<E>: Error,
{
	fn into_errors(self) -> Vec<Message<'static>> {
		match self {
			Self::App(error) => error.into_errors(),
			Self::Route(error) => error.into_errors(),
		}
	}

	fn status(&self) -> StatusCode {
		match self {
			Self::App(error) => error.status(),
			Self::Route(error) => error.status(),
		}
	}
}

/// Single error message shape.
///
/// Represents a single error message, often accompanied by
/// additional context (e.g. when a validation error displays
/// requirements).
#[derive(Debug, Serialize, JsonSchema)]
pub struct Message<'e> {
	pub code: Cow<'e, str>,
	#[serde(skip_serializing_if = "Option::is_none", rename = "message")]
	pub content: Option<Cow<'e, str>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub details: Option<Cow<'e, Map<Cow<'e, str>, serde_json::Value>>>,
}

impl<'e> Message<'e> {
	pub fn new(code: impl Into<Cow<'e, str>>) -> Self {
		Self {
			code: code.into(),
			content: None,
			details: None,
		}
	}

	pub fn detail(
		mut self,
		key: impl Into<Cow<'e, str>>,
		value: impl Into<serde_json::Value>,
	) -> Self {
		self.details
			.get_or_insert_with(Cow::default)
			.to_mut()
			.insert(key.into(), value.into());

		self
	}

	pub fn content(mut self, content: impl Into<Cow<'e, str>>) -> Self {
		self.content = Some(content.into());
		self
	}

	pub fn into_vec(self) -> Vec<Self> {
		vec![self]
	}
}
