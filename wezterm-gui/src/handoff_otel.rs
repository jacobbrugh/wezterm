// OpenTelemetry instrumentation for the wezterm-gui startup handoff path.
//
// Initializes an OTLP/HTTP span exporter and exposes a `HandoffSpan` guard
// that wraps the per-startup handoff decision flow. Events and attributes
// are set on the current span from within `Publish::resolve` and
// `Publish::try_spawn`, so the full decision trace for a single hyper+T
// press lands as one span in VictoriaTraces.
//
// Exporter target: `$OTEL_EXPORTER_OTLP_ENDPOINT/v1/traces`, or
// `https://otlp-push.jacobbrugh.net/v1/traces` by default. The
// SimpleSpanProcessor exports synchronously with reqwest-blocking, so no
// async runtime is needed.
//
// The SDK is opt-out: setting `WEZTERM_OTEL_DISABLE=1` or hitting any
// initialization error (e.g. offline host) silently falls back to no-op
// tracing; the existing `log::info!(target: "wezterm::handoff", …)` calls
// still cover the same ground and ship via the log-file-tailing pipeline.

use opentelemetry::trace::{SpanKind, Status, TraceContextExt, Tracer};
use opentelemetry::{Context, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::{SimpleSpanProcessor, TracerProvider};
use opentelemetry_sdk::Resource;
use std::sync::OnceLock;

const DEFAULT_ENDPOINT: &str = "https://otlp-push.jacobbrugh.net";
const TRACER_NAME: &str = "wezterm::handoff";

static PROVIDER: OnceLock<Option<TracerProvider>> = OnceLock::new();

/// Initialize the global tracer provider. Idempotent; safe to call from
/// `main()` early. Returns whether OTel was actually wired up — callers
/// can log but shouldn't change behavior based on it.
pub fn init() -> bool {
    PROVIDER
        .get_or_init(|| {
            if std::env::var_os("WEZTERM_OTEL_DISABLE").is_some() {
                return None;
            }
            let base = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string());
            let endpoint = format!("{}/v1/traces", base.trim_end_matches('/'));

            let exporter = match opentelemetry_otlp::SpanExporter::builder()
                .with_http()
                .with_endpoint(endpoint)
                .with_protocol(opentelemetry_otlp::Protocol::HttpJson)
                .build()
            {
                Ok(e) => e,
                Err(err) => {
                    log::warn!(
                        target: "wezterm::handoff",
                        "pid={} OTel init failed (falling back to no-op): {err:#}",
                        std::process::id()
                    );
                    return None;
                }
            };

            let provider = TracerProvider::builder()
                .with_span_processor(SimpleSpanProcessor::new(Box::new(exporter)))
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", "wezterm-gui"),
                    KeyValue::new(
                        "service.instance.id",
                        std::process::id().to_string(),
                    ),
                ]))
                .build();

            opentelemetry::global::set_tracer_provider(provider.clone());
            Some(provider)
        })
        .is_some()
}

/// RAII guard wrapping the startup handoff span. Drop ends the span.
/// While it's in scope, `event(...)` and `set_attr(...)` record onto the
/// current span via the attached `Context`.
pub struct HandoffSpan {
    _ctx_guard: opentelemetry::ContextGuard,
}

impl HandoffSpan {
    pub fn start(initial_attrs: Vec<KeyValue>) -> Self {
        let tracer = opentelemetry::global::tracer(TRACER_NAME);
        let span = tracer
            .span_builder("run_terminal_gui.handoff")
            .with_kind(SpanKind::Internal)
            .with_attributes(initial_attrs)
            .start(&tracer);
        let ctx = Context::current_with_span(span);
        Self {
            _ctx_guard: ctx.attach(),
        }
    }
}

/// Record a named event on the active handoff span (no-op if no span is
/// active or OTel is disabled).
pub fn event(name: &'static str, attrs: Vec<KeyValue>) {
    let ctx = Context::current();
    let span = ctx.span();
    span.add_event(name, attrs);
}

/// Set an attribute on the active handoff span.
pub fn set_attr(kv: KeyValue) {
    let ctx = Context::current();
    let span = ctx.span();
    span.set_attribute(kv);
}

/// Mark the active handoff span as failed with the given error message.
pub fn set_error(err: &str) {
    let ctx = Context::current();
    let span = ctx.span();
    span.set_status(Status::error(err.to_string()));
}

/// Flush and shut down the exporter. Call on clean exit.
pub fn shutdown() {
    if let Some(Some(provider)) = PROVIDER.get() {
        if let Err(err) = provider.shutdown() {
            log::warn!(
                target: "wezterm::handoff",
                "pid={} OTel shutdown error: {err:#}",
                std::process::id()
            );
        }
    }
}
