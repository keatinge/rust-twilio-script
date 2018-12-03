extern crate hyper;
extern crate hyper_tls;
extern crate serde_json;
extern crate futures;
extern crate url;
extern crate tokio_core;


use self::hyper::header::{Authorization, Basic, ContentType};
use self::futures::{Future, Stream};


pub struct Twilio {
    sid: String,
    auth: String,
    hyper_client: hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>,
}



#[derive(Debug)]
pub enum TwilioResponseError {
    HttpRequestError(hyper::error::Error),
    HttpStatusError(hyper::Response),
    CouldntParseJsonError(String, serde_json::error::Error)
}



impl Twilio {
    pub fn new(handle:&tokio_core::reactor::Handle) -> Twilio {
        let hyper_client = hyper::Client::configure()
            .connector(hyper_tls::HttpsConnector::new(1, handle).unwrap())
            .build(handle);
        Twilio {
            sid: "your_api_sid".to_owned(),
            auth : "your_api_auth".to_owned(),
            hyper_client,
        }
    }


    pub fn make_post_request(&self, endpoint: &str, body: String) -> hyper::client::FutureResponse {
        let url = format!("https://api.twilio.com/2010-04-01/Accounts/{}/{}.json", self.sid, endpoint);

        let mut req: hyper::Request<hyper::Body> = hyper::Request::new(hyper::Method::Post, url.parse().expect("Failed to parse url"));
        req.headers_mut().set(Authorization(Basic { username : self.sid.clone(), password: Some(self.auth.clone())}));
        req.headers_mut().set(ContentType::form_url_encoded());

        req.set_body(body);

        self.hyper_client.request(req)
    }


    /// Returns a future which represents a sent text message, on success it will evaluate to
    /// a serde_json::Value representing the json returned by the twilio api. It can fail for any
    /// of the following reasons: Sending the request failed, a network error, Twilio returned the
    /// incorrect status code (HTTP 201 is expected), or the json failed to parse
    pub fn send_text_message(&self, number: &str, msg:&str) -> impl Future<Item=serde_json::Value, Error=TwilioResponseError> {
        let body:String = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("To", number)
            .append_pair("From", "+your_twilio_phone")
            .append_pair("Body", msg)
            .finish();


        self.make_post_request("Messages", body)
            .map_err(|e| TwilioResponseError::HttpRequestError(e))
            .and_then(|resp| {
                if resp.status() != hyper::StatusCode::Created {
                    return Err(TwilioResponseError::HttpStatusError(resp));
                }
                Ok(resp)
            })
            .and_then(|resp| {
                // I don't even think this err case is possible, this could only happen if a malloc failed or something
                resp.body().concat2().map_err(|e| TwilioResponseError::HttpRequestError(e))
            })
            .and_then(|c2_body| {
                let bytes = c2_body.into_iter().collect::<Vec<u8>>();
                let s = String::from_utf8_lossy(&bytes).into_owned();

                match serde_json::from_str(&s) {
                    Ok(sj_val) => Ok(sj_val),
                    Err(sj_err) => Err(TwilioResponseError::CouldntParseJsonError(s, sj_err)),
                }
            })
    }

    pub fn start_call(&self, to: &str, callback_url: &str) -> impl Future<Item=serde_json::Value, Error=TwilioResponseError> {
        let body:String = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("To", to)
            .append_pair("From", "+your_twilio_phone")
            .append_pair("Url", callback_url)
            .finish();

        self.make_post_request("Calls", body)
            .map_err(|e| TwilioResponseError::HttpRequestError(e))
            .and_then(|resp| {
                if resp.status() != hyper::StatusCode::Created {
                    return Err(TwilioResponseError::HttpStatusError(resp));
                }
                Ok(resp)
            })
            .and_then(|resp| resp.body().concat2().map_err(|e| TwilioResponseError::HttpRequestError(e)))
            .and_then(|body_bytes| {
                let s = String::from_utf8_lossy(&body_bytes).into_owned();

                match serde_json::from_str(&s) {
                    Ok(sj_val) => Ok(sj_val),
                    Err(sj_err) => Err(TwilioResponseError::CouldntParseJsonError(s, sj_err)),
                }
            })
    }
}


