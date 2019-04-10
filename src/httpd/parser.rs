use alloc::{
    borrow::ToOwned,
    collections::BTreeMap,
    string::{String, ToString},
};

use super::request::Request;

pub struct HTTPParser {
    pub source: String,
    method: Option<String>,
    path: Option<String>,
    version: Option<String>,
    headers: BTreeMap<String, String>,
}

#[derive(Debug)]
pub enum ParseError {
    NotEnoughInput,
    Fatal,
}

impl HTTPParser {
    pub fn new(source: &str) -> HTTPParser {
        HTTPParser {
            source: source.to_string(),
            method: None,
            path: None,
            version: None,
            headers: BTreeMap::new(),
        }
    }

    pub fn parse_head(&mut self) -> Result<Request, ParseError> {
        self.parse_method()
            .and_then(|_| self.expect(" "))
            .and_then(|_| self.parse_path())
            .and_then(|_| self.expect(" "))
            .and_then(|_| self.parse_version())
            .and_then(|_| self.expect("\r\n"))
            .and_then(|_| self.parse_headers())
            .and_then(|_| self.expect("\r\n"))
            .and_then(|_| {
                Ok(Request::new(
                    self.method.to_owned().unwrap(),
                    self.path.to_owned().unwrap(),
                    self.version.to_owned().unwrap(),
                    self.headers.to_owned(),
                ))
            })
    }

    fn parse_method(&mut self) -> Result<String, ParseError> {
        let allowed_methods = ["GET", "POST"];
        let min_method_length = allowed_methods
            .iter()
            .map(|method| method.len())
            .min()
            .unwrap();

        if self.source.len() < min_method_length {
            return Err(ParseError::NotEnoughInput);
        }

        for method in allowed_methods.iter() {
            if self.source.starts_with(method) {
                return self.expect(method).map(|method| {
                    self.method = Some(method.to_owned());
                    method
                });
            }
        }

        Err(ParseError::Fatal)
    }

    fn parse_path(&mut self) -> Result<String, ParseError> {
        self.read_until(" ").map(|path| {
            self.path = Some(path.to_owned());
            path
        })
    }

    fn parse_version(&mut self) -> Result<String, ParseError> {
        self.read_until("\r\n").map(|version| {
            self.version = Some(version.to_owned());
            version
        })
    }

    fn parse_headers(&mut self) -> Result<(), ParseError> {
        loop {
            if self.source.starts_with("\r\n") {
                return Ok(());
            }

            match self.parse_header() {
                Ok((key, value)) => {
                    self.headers.insert(key, value);
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn parse_header(&mut self) -> Result<(String, String), ParseError> {
        let mut maybe_key = None;
        let mut maybe_value = None;

        self.read_until(":")
            .map(|key| maybe_key = Some(key.to_lowercase()))
            .and_then(|_| self.expect(":"))
            .and_then(|_| self.read_until("\r\n"))
            .map(|value| maybe_value = Some(value))
            .and_then(|_| self.expect("\r\n"))
            .map(|_| (maybe_key.unwrap(), maybe_value.unwrap().trim().to_string()))
    }

    fn expect(&mut self, expected: &str) -> Result<String, ParseError> {
        if expected.len() > self.source.len() {
            return Err(ParseError::NotEnoughInput);
        }

        if self.source.starts_with(expected) {
            let (init, rest) = self.source.split_at(expected.len());
            let init_as_str = init.to_string();
            let rest_as_str = rest.to_string();

            self.source = rest_as_str;

            return Ok(init_as_str);
        }

        Err(ParseError::Fatal)
    }

    /// Reads in source until delimiter.
    /// Does not consume delimiter.
    fn read_until(&mut self, delimiter: &str) -> Result<String, ParseError> {
        let mut already_read = String::new();

        // TODO: optimize, dont do as many string ops
        // maybe just keep the index and split at the end
        // There werent any library functions for that so i skipped it sorry
        loop {
            if self.source.len() < delimiter.len() {
                return Err(ParseError::NotEnoughInput);
            }

            if self.source.starts_with(delimiter) {
                return Ok(already_read);
            }

            let (first, rest) = self.source.split_at(1);

            already_read.push_str(first);
            self.source = rest.to_string();
        }
    }
}
