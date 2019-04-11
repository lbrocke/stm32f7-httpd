//! Custom logger indented for use with the STM32F7-Discovery board

//! The logger prints to the host stdout as well as the microcontroller LCD.
//! This of course requires the LCD display to be set up beforehand. This means
//! that one of the two LCD layers should have been initialited for use with
//! stdout:
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
//!     //   "[  15.32] This is an informational message"
//!     info!("This is an informational message");
//!
//!     // prints something like
//!     //   "[  15.33] (Warn) This is a warning"
//!     warn!("This is a warning");
//! }
//! ```

extern crate log;

use alloc::string::String;
use alloc::vec::Vec;
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use stm32f7_discovery::{lcd, print, println, system_clock};

struct Stm32f7Logger;

static LOGGER: Stm32f7Logger = Stm32f7Logger;

const NUM_COLS: usize = 60;

impl log::Log for Stm32f7Logger {
    // This logger implements all levels, therefore always return true
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let clock = system_clock::ms() as u64;
        let sec = clock / 1000;
        let ms = (clock - sec * 1000) / 10;

        // see module description above for format explanation
        let prefix = format!("[{:>4}.{:>02}] ", sec, ms);

        // print log level for every level except Info
        let lvl = match record.level() {
            Level::Info => String::from(""),
            _ => format!("({:?}) ", record.level()),
        };

        let message = format!("{}{}", lvl, record.args());

        // print to host stdout (when not running in release
        #[cfg(debug_assertions)]
        {
            use core::fmt::Write;
            use cortex_m_semihosting::hio;
            if let Ok(mut hstdout) = hio::hstdout() {
                let _ = writeln!(hstdout, "{}{}", prefix, message);
            }
        }

        // print to LCD
        if lcd::stdout::is_initialized() {
            // split message into fixed-sized substrings
            let prefix_len = prefix.chars().count();
            let lines = split_string(&message, NUM_COLS - prefix_len);

            for line in 0..lines.len() {
                if line == 0 {
                    print!("{}", prefix);
                } else {
                    // indent following lines with whitespace
                    print!("{}", " ".repeat(prefix_len));
                }

                println!("{}", lines[line]);
            }
        }
    }

    fn flush(&self) {}
}

// splits a string into fixed-size substrings
fn split_string(string: &str, sub_len: usize) -> Vec<&str> {
    let mut subs = Vec::with_capacity(string.len() / sub_len);
    let mut iter = string.chars();
    let mut pos = 0;

    while pos < string.len() {
        let mut len = 0;
        for ch in iter.by_ref().take(sub_len) {
            len += ch.len_utf8();
        }
        subs.push(&string[pos..pos + len]);
        pos += len;
    }

    subs
}

// initialize logger and enable all logging levels
pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace))
}
