use super::status::Status;
use alloc::{collections::BTreeMap, string::String, vec::Vec};

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
