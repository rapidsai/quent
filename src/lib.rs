use opentelemetry_appender_log::OpenTelemetryLogBridge;
use opentelemetry_sdk::logs::BatchLogProcessor;
use opentelemetry_sdk::logs::SdkLoggerProvider;

#[cfg(test)]
mod test {

    use opentelemetry_otlp::WithExportConfig;

    use super::*;

    #[test]
    fn sandbox() -> Result<(), Box<dyn std::error::Error>> {
        // let exporter = opentelemetry_stdout::LogExporter::default();
        let exporter = opentelemetry_otlp::LogExporter::builder()
            .with_tonic()
            .build()?;

        let provider = SdkLoggerProvider::builder()
            .with_simple_exporter(exporter)
            .build();

        let appender = OpenTelemetryLogBridge::new(&provider);

        log::set_boxed_logger(Box::new(appender))?;

        // somehow log::info! doesn't work immediately.
        log::logger().log(&log::RecordBuilder::new().build());

        provider.force_flush()?;
        provider.shutdown()?;

        Ok(())
    }
}
