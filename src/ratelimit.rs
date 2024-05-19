use std::{sync::Arc, time::Duration};

use axum::{
	body::Body,
	response::{IntoResponse, Response},
};
use governor::{
	clock::QuantaInstant,
	middleware::{RateLimitingMiddleware, StateInformationMiddleware},
};
use tower_governor::{
	governor::{GovernorConfig, GovernorConfigBuilder},
	key_extractor::{KeyExtractor, PeerIpKeyExtractor},
	GovernorError,
};

pub fn default() -> Arc<GovernorConfig<PeerIpKeyExtractor, StateInformationMiddleware>> {
	Arc::new(
		GovernorConfigBuilder::default()
			.per_second(10)
			.burst_size(50)
			.use_headers()
			.error_handler(error_handler)
			.finish()
			.unwrap(),
	)
}

pub fn secure() -> Arc<GovernorConfig<PeerIpKeyExtractor, StateInformationMiddleware>> {
	Arc::new(
		GovernorConfigBuilder::default()
			.per_second(1)
			.use_headers()
			.error_handler(error_handler)
			.finish()
			.unwrap(),
	)
}

fn error_handler(error: GovernorError) -> Response<Body> {
	crate::Error::from(error).into_response()
}

pub fn cleanup_old_limits<T, M>(configs: &[&Arc<GovernorConfig<T, M>>])
where
	T: KeyExtractor,
	<T as KeyExtractor>::Key: Send + Sync + 'static,
	M: RateLimitingMiddleware<QuantaInstant> + Send + Sync + 'static,
{
	let limiters = configs
		.iter()
		.map(|config| config.limiter().clone())
		.collect::<Vec<_>>();
	let interval = Duration::from_secs(60);

	std::thread::spawn(move || loop {
		std::thread::sleep(interval);

		for limiter in &limiters {
			tracing::debug!("rate limiting storage size: {}", limiter.len());

			limiter.retain_recent();
		}
	});
}
