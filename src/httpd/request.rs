use alloc::{
    collections::BTreeMap,
    string::{String, ToString}
};

#[derive(Debug)]
pub struct Request {
    method: String,
    path: String,
    version: String,
    headers: BTreeMap<String, String>
}

impl Request {
    pub fn new(method: String, path: String, version: String, headers: BTreeMap<String, String>) -> Request {
        Request {
            method,
            path,
            version,
            headers,
        }
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn get_header(&self, key: String) -> Option<String> {
        self.headers.get(&key).map(|maybe_val| maybe_val.to_string())
    }
}
