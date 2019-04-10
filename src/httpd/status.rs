#[derive(Clone, Debug)]
pub enum Status {
    OK,
}

impl Status {
    pub fn numerical_and_text(&self) -> (u16, &str) {
        match self {
            Status::OK => (200, "OK"),
        }
    }
}
