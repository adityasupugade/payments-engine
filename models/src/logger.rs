use std::path::PathBuf;
use std::sync::Once;
use tracing_subscriber::fmt;
static LOGGER: Once = Once::new();


pub struct Logger {
    logs_dir_path: PathBuf,
    filter: String,
}

impl Logger {
    pub fn new(logs_dir_path: PathBuf, filter: String) -> Self {
        Self { 
            logs_dir_path, 
            filter,
        }
    }

    pub fn start(&self) {
        LOGGER.call_once(|| {
            let log_file_dir = self.logs_dir_path.clone();
            let file_appender = tracing_appender::rolling::daily(log_file_dir, "engine.log");
            let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

            let filter = match std::env::var("ENGINE_LOG") {
                Ok(val) => {
                    tracing_subscriber::filter::EnvFilter::from(val)
                },
                Err(_) => {
                    tracing_subscriber::filter::EnvFilter::from(self.filter.as_str())
                }
            };

            tracing::subscriber::set_global_default(
                fmt::Subscriber::builder()
                    .with_writer(file_writer)
                    .with_env_filter(filter)
                    .with_ansi(false)
                    .finish()
            ).expect("Unable to set the global tracing default");

            std::mem::forget(guard);
        });
    }
}