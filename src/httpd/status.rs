#[derive(Clone, Debug)]
pub enum Status {
    // 200
    OK,

    // 400
    NotFound,
}

impl Status {
    pub fn numerical_and_text(&self) -> (u16, &str) {
        match self {
            Status::OK => (200, "OK"),

            Status::NotFound => (404, "Not found"),
        }
    }
}
