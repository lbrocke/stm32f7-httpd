pub enum Status {
    OK
}

impl Status {
    pub fn to_numerical(&self) -> u16 {
        match self {
            Status::OK => 200
        }
    }
}
