extern crate hyper;
extern crate futures;

use hyper::header::ContentLength;

pub fn not_allowed_error(text: &str) -> hyper::Response{
    hyper::Response::new()
        .with_status(hyper::StatusCode::MethodNotAllowed)
        .with_header(ContentLength(text.len() as u64))
        .with_body(String::from(text))
}

pub fn bad_request_error(text: &str) -> hyper::Response {
    hyper::Response::new()
        .with_status(hyper::StatusCode::BadRequest)
        .with_header(ContentLength(text.len() as u64))
        .with_body(String::from(text))
}

