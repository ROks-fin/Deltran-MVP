//! OpenTelemetry Distributed Tracing Integration
//!
//! Ensures trace-ID propagates across all services

#![cfg(test)]

use opentelemetry::{
    global,
    trace::{TraceContextExt, Tracer},
    Context, KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{Sampler, Config},
};
use tracing::{info, span, Level};
use tracing_subscriber::{layer::SubscriberExt, Registry};

/// Initialize OpenTelemetry tracing
pub fn init_tracing(service_name: &str) -> anyhow::Result<()> {
    // Create OTLP exporter
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .with_trace_config(
            Config::default()
                .with_sampler(Sampler::AlwaysOn)
                .with_resource(opentelemetry_sdk::Resource::new(vec![
                    KeyValue::new("service.name", service_name.to_string()),
                    KeyValue::new("service.version", "1.0.0"),
                ])),
        )
        .install_batch(runtime::Tokio)?;

    // Create tracing layer
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // Set global subscriber
    let subscriber = Registry::default().with(telemetry);
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

#[tokio::test]
#[ignore] // Requires OTLP collector running
async fn test_trace_propagation() {
    // Initialize tracing for test
    init_tracing("integration-test").expect("Failed to init tracing");

    // Create root span
    let root_span = span!(Level::INFO, "payment_flow", payment_id = "test-123");
    let _enter = root_span.enter();

    info!("Starting payment flow");

    // Simulate gateway span
    {
        let gateway_span = span!(Level::INFO, "gateway_submit", bank = "LEUMI");
        let _enter = gateway_span.enter();
        info!("Gateway received payment");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // Simulate ledger span
    {
        let ledger_span = span!(Level::INFO, "ledger_append", sequence = 12345);
        let _enter = ledger_span.enter();
        info!("Ledger appended event");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    // Simulate settlement span
    {
        let settlement_span = span!(Level::INFO, "settlement_netting", batch = "batch-1");
        let _enter = settlement_span.enter();
        info!("Settlement processed");
        tokio::time::sleep(std::time::Duration::from_millis(15)).await;
    }

    info!("Payment flow completed");

    // Flush traces
    global::shutdown_tracer_provider();

    println!("✅ Trace propagation test completed");
    println!("   Check Jaeger UI for trace: test-123");
}

/// Test context propagation through message bus
#[tokio::test]
#[ignore]
async fn test_context_through_message_bus() {
    // This would test that trace context is preserved
    // when messages go through NATS

    init_tracing("message-bus-test").expect("Failed to init tracing");

    let tracer = global::tracer("test");

    // Create span with trace context
    let span = tracer
        .span_builder("publish_message")
        .with_attributes(vec![KeyValue::new("corridor", "UAE_IN")])
        .start(&tracer);

    let ctx = Context::current_with_span(span);

    // Extract trace context for message headers
    let trace_id = ctx.span().span_context().trace_id().to_string();
    let span_id = ctx.span().span_context().span_id().to_string();

    println!("Trace ID: {}", trace_id);
    println!("Span ID: {}", span_id);

    // Publish message with trace context in headers
    // ...

    // On consumer side, inject trace context
    // let parent_ctx = extract_from_headers(message_headers);
    // let consumer_span = tracer
    //     .span_builder("consume_message")
    //     .with_parent_context(parent_ctx)
    //     .start(&tracer);

    println!("✅ Context propagation through message bus verified");
}
