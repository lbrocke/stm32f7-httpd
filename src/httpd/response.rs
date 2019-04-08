use alloc::{
    collections::BTreeMap,
    string::String
};
use super::status::Status;

pub struct Response {
    status: Status,
    headers: BTreeMap<String, String>,
    body: String
}

impl Response {
    pub fn new() -> Response {
        Response { status : Status::OK, headers : BTreeMap::new(), body : String::new() }
    }

    pub fn status(&mut self, status: Status) {
        unimplemented!();
    }

    pub fn header(&mut self, key: &str, value: &str) {
        unimplemented!();
    }

    pub fn write(&mut self, chunk: &str) {
        unimplemented!();
    }

    pub fn end(&mut self) {
        unimplemented!();
    }
}
