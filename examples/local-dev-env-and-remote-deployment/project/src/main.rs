use axum::{response::Html, routing::get, Json, Router};
use color_eyre::eyre::{self, WrapErr};
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::runtime;
use serde::Serialize;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    dotenv::dotenv().ok();

    init_tracing()?;

    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("{host}:{port}");

    let app = Router::new().route("/", get(index)).layer(
        TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO)),
    );

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .wrap_err(format!("Failed to bind to '{addr}'"))?;

    tracing::info!("Listening on {addr}");

    axum::serve(listener, app).await.wrap_err("Server error")
}

#[tracing::instrument]
async fn index() -> Html<&'static str> {
    tracing::info!("Serving index page");
    Html(INDEX_HTML)
}

static DEFAULT_OTLP_ENDPOINT: &str = "http://localhost:4318";

fn init_tracing() -> eyre::Result<()> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| DEFAULT_OTLP_ENDPOINT.into());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(&endpoint)
        .build()
        .wrap_err("Failed to build OTLP exporter")?;

    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", "webservice"),
        ]))
        .build();

    let tracer = provider.tracer("webservice");
    opentelemetry::global::set_tracer_provider(provider);

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(otel_layer)
        .init();

    tracing::info!(endpoint, "OpenTelemetry exporter initialized");

    Ok(())
}

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>webservice</title>
    <style>
        body {
            font-family: system-ui, sans-serif;
            background: #1a1a2e;
            color: #eee;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            margin: 0;
        }
        .container { text-align: center; }
        h1 { font-size: 2rem; margin-bottom: 0.5rem; }
        p { color: #888; }
        a { color: #7b68ee; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Hello from webserver!</h1>
        <p>Provisioned with <a href="https://github.com/myFavShrimp/arc">arc</a></p>
    </div>
</body>
</html>"#;
