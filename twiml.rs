extern crate hyper;

use std::convert::From;
use hyper::mime;
use hyper::header::{Basic, ContentType, ContentLength};

pub struct Twiml {
    data: String,
}


impl From<Twiml> for hyper::Response {
    fn from(owned_twiml:Twiml) -> hyper::Response {
        hyper::Response::new()
            .with_header(ContentType(mime::TEXT_XML))
            .with_header(ContentLength(owned_twiml.data.len() as u64))
            .with_body(owned_twiml.data)
    }
}
impl From<String> for Twiml {
    fn from(owned_str:String) -> Twiml {
        Twiml {data : owned_str}
    }
}


pub fn get_input(callback_url: &str, to_say: &str) -> Twiml {
    Twiml::from(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
                <Response>
                    <Gather input="dtmf" timeout="10" numDigits="1" action="{}">
                        <Say voice="woman">{}</Say>
                    </Gather>
                </Response>
                "#, callback_url.replace("&", "&amp;"), to_say))
}

pub fn say(to_say: &str) -> Twiml {
    Twiml::from(format!(r#"<?xml version="1.0" encoding="UTF-8"?>
                <Response>
                    <Say voice="woman">{}</Say>
                </Response>
                "#, to_say))
}