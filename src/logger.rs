use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use env_logger::{fmt, Builder, Env, Target};
use log::{Level, Record};
use serde_json::json;

// Initialize simple logger: JSON to Stdout
pub fn init() {
    let env = Env::default()
        .default_filter_or("info")
        .default_write_style_or("never");

    Builder::from_env(env)
        .target(Target::Stdout)
        .format(log_format)
        .init();
}

// Log record format function
fn log_format(buf: &mut fmt::Formatter, record: &Record) -> io::Result<()> {
    let level = record.level();
    let level_human = match level {
        Level::Warn => "warn",
        Level::Error => "error",
        Level::Info => "info",
        Level::Debug => "debug",
        Level::Trace => "trace",
    };
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    write!(buf, "{{")?;
    write!(buf, "\"level\":\"{}\"", level_human)?;
    write!(buf, ",\"time\":{}", time)?;
    write!(buf, ",\"msg\":{}", json!(record.args().to_string()))?;
    if level == log::Level::Debug || level == log::Level::Trace {
        write!(buf, ",\"target\":\"{}\"", record.target())?;
        if let Some(module) = record.module_path() {
            write!(buf, ",\"module\":\"{}\"", module)?;
        }
        if let Some(file) = record.file() {
            write!(buf, ",\"file\":\"{}\"", file)?;
        }
        if let Some(line) = record.line() {
            write!(buf, ",\"line\":\"{}\"", line)?;
        }
    }
    writeln!(buf, "}}")
}
