//! Custom logger

extern crate log;

use alloc::string::String;
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use stm32f7_discovery::{print, println, system_clock};

struct Stm32F7Logger;

static LOGGER: Stm32F7Logger = Stm32F7Logger;

impl log::Log for Stm32F7Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let clock = system_clock::ms() as u64;
            let sec = clock / 1000;
            let ms = clock - sec * 1000;

            let s = match record.level() {
                Level::Info => String::from(""),
                _ => format!("({:?}) ", record.level()),
            };

            println!("[{:>4}.{:>03}] {}{}", sec, ms, s, record.args());
        }
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace))
}
