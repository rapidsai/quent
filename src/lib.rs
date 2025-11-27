use opentelemetry_appender_log::OpenTelemetryLogBridge;
use opentelemetry_sdk::logs::BatchLogProcessor;
use opentelemetry_sdk::logs::SdkLoggerProvider;

#[cfg(test)]
mod test {

    use log::{Level, Record};
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;

    use super::*;

    // #[tokio::test]
    #[test]
    fn sandbox() -> Result<(), Box<dyn std::error::Error>> {
        let rt = tokio::runtime::Builder::new_current_thread().build()?;
        // let exporter = opentelemetry_stdout::LogExporter::default();
        // let exporter = opentelemetry_otlp::LogExporter::builder()
        // .with_tonic()
        // .build()?;
        let exporter = opentelemetry_otlp::LogExporter::builder()
            .with_http()
            .build()?;

        let provider = SdkLoggerProvider::builder()
            .with_simple_exporter(exporter)
            .build();

        let appender = OpenTelemetryLogBridge::new(&provider);

        log::set_boxed_logger(Box::new(appender))?;

        println!(
            "OTEL_LOGS_EXPORTER={:?}",
            std::env::var("OTEL_LOGS_EXPORTER")
        );
        println!(
            "OTEL_EXPORTER_OTLP_ENDPOINT={:?}",
            std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        );
        println!("logging...");
        // somehow log::info! doesn't work immediately.
        log::logger().log(
            &Record::builder()
                .args(format_args!(
                    "amanalap amanalap amanalap amanalap amanalap amanalap amanalap"
                ))
                .level(Level::Error)
                .target("myApp")
                .file(Some("server.rs"))
                .line(Some(144))
                .module_path(Some("server"))
                .build(),
        );

        println!("flushing...");
        provider.force_flush()?;

        println!("shutting down...");
        provider.shutdown()?;

        Ok(())
    }
}
