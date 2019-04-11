use super::status::Status;
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

#[derive(Clone, Debug)]
pub struct Response {
    pub status: Status,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: Status, headers: BTreeMap<String, String>, body: Vec<u8>) -> Response {
        Response {
            status,
            headers,
            body,
        }
    }
}

pub struct ResponseBuilder {
    status: Status,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

impl ResponseBuilder {
    pub fn new(status: Status) -> ResponseBuilder {
        ResponseBuilder {
            status,
            headers: BTreeMap::new(),
            body: vec![],
        }
    }

    pub fn header<KType: ToString, VType: ToString>(mut self, key: KType, value: VType) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    pub fn body_html(mut self, body_str: &str) -> Self {
        self.headers
            .insert("Content-Type".to_string(), "text/html".to_string());
        self.body = body_str.into();

        self
    }

    pub fn finalize(mut self) -> Response {
        if self.body.len() > 0 {
            self.headers
                .insert("Content-Length".into(), self.body.len().to_string());

            if !self.headers.contains_key("Content-Type") {
                self.headers.insert(
                    "Content-Type".into(),
                    "application/octet-stream".to_string(),
                );
            }
        }

        Response::new(self.status, self.headers, self.body)
    }
}
