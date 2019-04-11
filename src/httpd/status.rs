#[derive(Clone, Debug)]
pub enum Status {
    // 200
    OK,

    // 400
    BadRequest,
    NotFound,
}

impl Status {
    pub fn numerical_and_text(&self) -> (u16, &str) {
        match self {
            Status::OK => (200, "OK"),

            Status::BadRequest => (400, "Bad Request"),
            Status::NotFound => (404, "Not Found"),
        }
    }
}
