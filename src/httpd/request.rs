use alloc::string::String;

#[derive(Debug)]
pub struct Request {
    method: String,
    path: String,
    version: String,
}

impl Request {
    pub fn new(method: String, path: String, version: String) -> Request {
        Request {
            method,
            path,
            version,
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
}
