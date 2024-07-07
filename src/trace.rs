use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
	runtime,
	trace::{BatchConfig, Sampler, Tracer},
	Resource,
};
use opentelemetry_semantic_conventions::{
	resource::{DEPLOYMENT_ENVIRONMENT, SERVICE_NAME, SERVICE_VERSION},
	SCHEMA_URL,
};
use tracing::{level_filters::LevelFilter, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Constructs a [`Resource`] which describes the service.
fn resource() -> Resource {
	Resource::from_schema_url(
		[
			KeyValue::new(SERVICE_NAME, env!("CARGO_PKG_NAME")),
			KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
			KeyValue::new(
				DEPLOYMENT_ENVIRONMENT,
				if cfg!(debug_assertions) {
					"development"
				} else {
					"production"
				},
			),
		],
		SCHEMA_URL,
	)
}

/// Constructs a [`Tracer`] with a custom sampling strategy and exporter.
fn init_tracer() -> Tracer {
	opentelemetry_otlp::new_pipeline()
		.tracing()
		.with_trace_config(
			opentelemetry_sdk::trace::Config::default()
				.with_sampler(Sampler::TraceIdRatioBased(1.0))
				.with_resource(resource()),
		)
		.with_batch_config(BatchConfig::default())
		.with_exporter(
			opentelemetry_otlp::new_exporter()
				.tonic()
				.with_endpoint(crate::env!("OTEL_EXPORTER_ENDPOINT")),
		)
		.install_batch(runtime::Tokio)
		.unwrap()
}

/// Initializes the tracing subscriber with OpenTelemetry support, returning
/// a guard that cleans up the global tracer and meter provider when dropped.
pub fn init_tracing_subscriber() -> OtelGuard {
	tracing_subscriber::registry()
		.with(LevelFilter::from_level(Level::INFO))
		.with(tracing_subscriber::fmt::layer().with_ansi(true))
		.with(tracing_opentelemetry::layer().with_tracer(init_tracer()))
		.init();

	OtelGuard
}

pub struct OtelGuard;

impl Drop for OtelGuard {
	fn drop(&mut self) {
		opentelemetry::global::shutdown_tracer_provider();
	}
}
