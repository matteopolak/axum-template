use std::{fs::read_to_string, path::PathBuf};

fn default_theme() -> String {
	"default".into()
}

fn default_spec_url() -> String {
	"/docs/private/api.json".into()
}

#[derive(serde::Deserialize)]
struct Config {
	package: Package,
}

#[derive(serde::Deserialize)]
struct Package {
	metadata: Metadata,
}

#[derive(serde::Deserialize)]
struct Metadata {
	scalar: Scalar,
}

#[derive(serde::Deserialize)]
struct Scalar {
	#[serde(default = "default_theme")]
	theme: String,
	#[serde(default = "default_spec_url")]
	spec_url: String,
	title: String,
}

fn main() {
	println!("cargo:rerun-if-changed=Cargo.toml");
	println!("cargo:rerun-if-changed=assets/scalar.min.js");

	let config = read_to_string("Cargo.toml").unwrap();
	let js = read_to_string("assets/scalar.min.js").unwrap();

	let config = toml::from_str::<Config>(&config).unwrap();
	let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());

	let html = format!(
		r#"<!DOCTYPE html>
		<html>
			<head>
				<title>{title}</title>
				<meta charset="utf-8" />
				<meta
					name="viewport"
					content="width=device-width, initial-scale=1" />
				<style>
					body {{
						margin: 0;
					}}
				</style>
			</head>
			<body>
				<script
					id="api-reference"></script>
				<script>
					const configuration = {{
						theme: {theme:?},
						spec: {{
							url: {spec_url:?}
						}}
					}};

					document.getElementById('api-reference')
						.dataset.configuration = JSON.stringify(configuration);
				</script>
				<script>
					{js}
				</script>
			</body>
		</html>"#,
		title = config.package.metadata.scalar.title,
		theme = config.package.metadata.scalar.theme,
		spec_url = config.package.metadata.scalar.spec_url,
	);

	std::fs::write(out.join("scalar.html"), html).unwrap();
}
