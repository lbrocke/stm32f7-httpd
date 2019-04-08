//! Custom logger indented for use with the STM32F7-Discovery board

//! IMPORTANT: This logger requires the LCD display to be set up beforehand.
//! This means that one of the two LCD layers should have been initialited for
//! use with stdout:
//!
//! ```
//! lcd::init_stdout(layer_2);
//! ```

//! This logger prefixes the given message with the ticks since startup in
//! [SECONDS.MILLIS] format, similar to the `dmesg` utility on Unix systems.
//! All log levels (error!, warn!, info!, debug!, trace!) are supported. For
//! every level except Info, the log level itself is additionally printed as
//! prefix of the given message.

//! To use this module, add the `log` crate to your Cargo.toml, add it to your
//! main file and call the init function:
//!
//! ```
//! mod logger;
//!
//! use log::{info, warn}; /* desired log levels */
//!
//! fn main() {
//!     logger::init().unwrap();
//!
//!     // prints something like
//!     //   "[   15.325] This is an informational message"
//!     info!("This is an informational message");
//!
//!     // prints something like
//!     //   "[   15.331] (Warn) This is a warning"
//!     warn!("This is a warning");
//! }
//! ```

extern crate log;

use alloc::string::String;
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use stm32f7_discovery::{print, println, system_clock};

struct Stm32f7Logger;

static LOGGER: Stm32f7Logger = Stm32f7Logger;

impl log::Log for Stm32f7Logger {
    // This logger implements all levels, therefore always return true
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let clock = system_clock::ms() as u64;
            let sec = clock / 1000;
            let ms = clock - sec * 1000;

            // print log level for every level except Info
            let lvl = match record.level() {
                Level::Info => String::from(""),
                _ => format!("({:?}) ", record.level()),
            };

            // see module description above for format explanation
            println!("[{:>4}.{:>03}] {}{}", sec, ms, lvl, record.args());
        }
    }

    fn flush(&self) {}
}

// initialize logger and enable all logging levels
pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace))
}
