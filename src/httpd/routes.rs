use super::request::Request;
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
};

pub enum Routes<'a, R> {
    NotMatched(&'a Request),
    Matched(R),
}

impl<'a, R> Routes<'a, R> {
    pub fn init<'b>(request: &'b Request) -> Routes<'b, R> {
        Routes::NotMatched(request)
    }

    pub fn route<F: FnOnce(&'a Request, BTreeMap<String, String>) -> R>(
        self,
        method: &str,
        pattern: &str,
        make_response: F,
    ) -> Routes<'a, R> {
        match self {
            Routes::Matched(result) => Routes::Matched(result),
            Routes::NotMatched(request) => {
                if method == request.method() {
                    let matched_path = match_path(pattern, &request.path());

                    match matched_path {
                        None => Routes::NotMatched(request),
                        Some(args) => Routes::Matched(make_response(request, args)),
                    }
                } else {
                    Routes::NotMatched(request)
                }
            }
        }
    }

    pub fn catch_all<F: FnOnce(&'a Request, BTreeMap<String, String>) -> R>(
        self,
        make_response: F,
    ) -> R {
        match self {
            Routes::Matched(result) => result,
            Routes::NotMatched(request) => make_response(request, BTreeMap::new()),
        }
    }
}

fn match_path(pattern: &str, path: &str) -> Option<BTreeMap<String, String>> {
    let mut pattern_parts = pattern.split("/");
    let mut path_parts = path.split("/");

    let mut args = BTreeMap::new();

    loop {
        match (pattern_parts.next(), path_parts.next()) {
            // If both iterators are empty, they were of equal size and no parts mismatched => we're done
            (None, None) => return Some(args),
            // If both still have a part, compare them and look for variables (starting with ":")
            (Some(pattern_part), Some(path_part)) => {
                if pattern_part.len() >= 2 && pattern_part.split_at(1).0 == ":" {
                    args.insert(
                        (pattern_part.split_at(1).1).to_string(),
                        path_part.to_string(),
                    );
                } else if pattern_part != path_part {
                    return None;
                }
            }
            // Otherwise, fail horribly
            _ => return None,
        }
    }
}
