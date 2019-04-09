use alloc::{
    collections::BTreeMap,
    string::String,
    vec::Vec,
};
use super::status::Status;

pub struct Response {
    status: Status,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

impl Response {
    pub fn new(status: Status, headers: BTreeMap<String, String>, body: Vec<u8>) -> Response {
        Response { status, headers, body }
    }
}