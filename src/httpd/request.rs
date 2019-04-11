use alloc::{collections::BTreeMap, string::String};

#[derive(Clone, Debug)]
pub struct Request {
    method: String,
    path: String,
    version: String,
    headers: BTreeMap<String, String>,
}

impl Request {
    pub fn new(
        method: String,
        path: String,
        version: String,
        headers: BTreeMap<String, String>,
    ) -> Request {
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

    #[allow(dead_code)]
    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn headers(&self) -> &BTreeMap<String, String> {
        &self.headers
    }
}
