use chrono::{DateTime, Utc};
use log::{Level, Metadata, Record};
pub struct MyLogger;
pub static MY_LOGGER: MyLogger = MyLogger;

impl log::Log for MyLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    #[cfg(not(target_family="wasm"))]
    fn log(&self, record: &Record) {
        let now: DateTime<Utc> = Utc::now();
        if self.enabled(record.metadata()) {
            println!(
                "[{}] {} - {}",
                now.to_rfc3339(),
                record.level(),
                record.args()
            );
        }
    }

    #[cfg(target_family="wasm")]
    fn log(&self, record: &Record) {
        use gloo_console::log as gloo_log;
        use js_sys::JsString;
        let now: DateTime<Utc> = Utc::now();
        if self.enabled(record.metadata()) {
            let str_log: JsString = format!(
                "[{}] {} - {}",
                now.to_rfc3339(),
                record.level(),
                record.args()
            )
            .into();
            gloo_log!(str_log);
        }
    }
    
    fn flush(&self) {}
}
