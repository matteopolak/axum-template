use opentelemetry::{global, Key, KeyValue};
use opentelemetry_sdk::{
	metrics::{
		reader::{DefaultAggregationSelector, DefaultTemporalitySelector},
		Aggregation, Instrument, MeterProviderBuilder, PeriodicReader, SdkMeterProvider, Stream,
	},
	runtime,
	trace::{BatchConfig, RandomIdGenerator, Sampler, Tracer},
	Resource,
};
use opentelemetry_semantic_conventions::{
	resource::{DEPLOYMENT_ENVIRONMENT, SERVICE_NAME, SERVICE_VERSION},
	SCHEMA_URL,
};
use tracing::Level;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Create a Resource that captures information about the entity for which telemetry is recorded.
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

// Construct MeterProvider for MetricsLayer
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
	let stdout_reader = PeriodicReader::builder(
		opentelemetry_stdout::MetricsExporter::default(),
		runtime::Tokio,
	)
	.build();

	// Set Custom histogram boundaries for baz metrics
	let view_latency = |instrument: &Instrument| -> Option<Stream> {
		println!("{:?}", instrument.name);

		if instrument.name == "request" {
			Some(Stream::new().name("request").allowed_attribute_keys([
				Key::from("request_id"),
				Key::from("method"),
				Key::from("uri"),
				Key::from("version"),
			]))
		} else {
			None
		}
	};

	let meter_provider = MeterProviderBuilder::default()
		.with_resource(resource())
		.with_reader(reader)
		.with_reader(stdout_reader)
		.with_view(view_latency)
		.build();

	global::set_meter_provider(meter_provider.clone());

	meter_provider
}

// Construct Tracer for OpenTelemetryLayer
fn init_tracer() -> Tracer {
	opentelemetry_otlp::new_pipeline()
		.tracing()
		.with_trace_config(
			opentelemetry_sdk::trace::Config::default()
				// Customize sampling strategy
				.with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
					1.0,
				))))
				// If export trace to AWS X-Ray, you can use XrayIdGenerator
				.with_id_generator(RandomIdGenerator::default())
				.with_resource(resource()),
		)
		.with_batch_config(BatchConfig::default())
		.with_exporter(opentelemetry_otlp::new_exporter().tonic())
		.install_batch(runtime::Tokio)
		.unwrap()
}

// Initialize tracing-subscriber and return OtelGuard for opentelemetry-related termination processing
pub fn init_tracing_subscriber() -> OtelGuard {
	let meter_provider = init_meter_provider();

	tracing_subscriber::registry()
		.with(tracing_subscriber::filter::LevelFilter::from_level(
			Level::INFO,
		))
		.with(tracing_subscriber::fmt::layer().with_ansi(true))
		.with(MetricsLayer::new(meter_provider.clone()))
		.with(OpenTelemetryLayer::new(init_tracer()))
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
