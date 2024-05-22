use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::{
	metrics::{
		reader::{DefaultAggregationSelector, DefaultTemporalitySelector},
		Aggregation, Instrument, MeterProviderBuilder, PeriodicReader, SdkMeterProvider, Stream,
	},
	runtime,
	trace::{BatchConfig, Sampler, Tracer},
	Resource,
};
use opentelemetry_semantic_conventions::{
	resource::{DEPLOYMENT_ENVIRONMENT, SERVICE_NAME, SERVICE_VERSION},
	SCHEMA_URL,
};
use tracing::{level_filters::LevelFilter, Level};
use tracing_opentelemetry::MetricsLayer;
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

/// Constructs an [`SdkMeterProvider`] with a custom view for latency metrics.
fn init_meter_provider() -> SdkMeterProvider {
	let exporter = opentelemetry_otlp::new_exporter()
		.tonic()
		.build_metrics_exporter(
			Box::new(DefaultAggregationSelector::new()),
			Box::new(DefaultTemporalitySelector::new()),
		)
		.unwrap();

	let reader = PeriodicReader::builder(exporter, runtime::Tokio)
		.with_interval(std::time::Duration::from_secs(5))
		.build();

	// For debugging in development
	#[cfg(debug_assertions)]
	let stdout_reader = PeriodicReader::builder(
		opentelemetry_stdout::MetricsExporter::default(),
		runtime::Tokio,
	)
	.build();

	// Set Custom histogram boundaries for baz metrics
	let view_latency = |instrument: &Instrument| -> Option<Stream> {
		if instrument.name == "latency_ms" {
			Some(
				Stream::new()
					.name("latency_ms")
					.aggregation(Aggregation::Default),
			)
		} else {
			None
		}
	};

	let meter_provider = MeterProviderBuilder::default();
	#[cfg(debug_assertions)]
	let meter_provider = meter_provider.with_reader(stdout_reader);

	let meter_provider = meter_provider
		.with_resource(resource())
		.with_reader(reader)
		.with_view(view_latency)
		.build();

	global::set_meter_provider(meter_provider.clone());

	meter_provider
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
		.with_exporter(opentelemetry_otlp::new_exporter().tonic())
		.install_batch(runtime::Tokio)
		.unwrap()
}

/// Initializes the tracing subscriber with OpenTelemetry support, returning
/// a guard that cleans up the global tracer and meter provider when dropped.
pub fn init_tracing_subscriber() -> OtelGuard {
	let meter_provider = init_meter_provider();

	tracing_subscriber::registry()
		.with(LevelFilter::from_level(Level::INFO))
		.with(tracing_subscriber::fmt::layer().with_ansi(true))
		.with(MetricsLayer::new(meter_provider.clone()))
		.with(tracing_opentelemetry::layer().with_tracer(init_tracer()))
		.init();

	OtelGuard { meter_provider }
}

pub struct OtelGuard {
	meter_provider: SdkMeterProvider,
}

impl Drop for OtelGuard {
	fn drop(&mut self) {
		if let Err(err) = self.meter_provider.shutdown() {
			eprintln!("{err:?}");
		}

		opentelemetry::global::shutdown_tracer_provider();
	}
}
