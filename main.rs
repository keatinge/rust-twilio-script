#![feature(conservative_impl_trait)]
extern crate tokio_core;
extern crate futures;
extern crate regex;
extern crate hyper;
extern crate url;

mod twil_api;
mod script;
mod ctxmgr;
mod twiml;
mod responses;

use std::collections::HashMap;
use futures::{Future, Stream};





#[derive(Debug)]
struct ExampleUserContext {
    f_name: String,
    l_name : String
}

impl ctxmgr::Context for ExampleUserContext {
    fn resolve_variable<'a>(&'a self, var:&str) -> Option<&'a str> {
        match var {
            "f_name" => Some(&self.f_name),
            "l_name" => Some(&self.l_name),
            _ => None,

        }
    }

    fn list_vars<'a, 'b : 'a>(&'a self) -> &'b[&'b str] {
        &["f_name", "l_name"]
    }

    fn from_kvs(mut hm:HashMap<String, String>) -> Self { // Todo return Result<Self>?
        let f_name = hm.remove("f_name").unwrap();
        let l_name = hm.remove("l_name").unwrap();

        ExampleUserContext { f_name, l_name}
    }
}


fn template<T>(raw: &str, ctx: &T) -> String where T: ctxmgr::Context {
    let mut owned_copy = String::from(raw);
    for var_name in ctx.list_vars().iter() {
        let to_replace = format!("{{{}}}", var_name);
        let substitution:&str = ctx.resolve_variable(var_name).expect("CTX was unable to resolve variable");
        owned_copy = owned_copy.replace(&to_replace, substitution);
    }
    owned_copy
}




struct TwilioResponseService<T> where T : ctxmgr::Context {
    sb_ptr: std::rc::Rc<script::ScriptBase>,
    ctx_ptr: std::rc::Rc<std::cell::RefCell<ctxmgr::ContextManager<T>>>, // This could/should be RwLock a if multithreaded
    pub_url: String
}

impl<T> TwilioResponseService<T> where T: ctxmgr::Context + std::fmt::Debug + 'static {
    fn handle_twilio(&self, req: hyper::Request) -> <Self as hyper::server::Service>::Future {
        let (method, uri, _, headers, body) = req.deconstruct();

        if uri.query().is_none() {
            return Box::new(futures::future::ok(responses::bad_request_error("Missing uri query")));
        }
        let sb_ptr_clone = std::rc::Rc::clone(&self.sb_ptr);
        let ctx_ptr_clone = std::rc::Rc::clone(&self.ctx_ptr);
        let url_clone = self.pub_url.clone();
        let result = Box::new(body.concat2().and_then(move |bytes_vec| {
            let qs = uri.query().unwrap();
            let qs_parsed_kvs = url::form_urlencoded::parse(qs.as_bytes()).into_owned().collect::<HashMap<String, String>>();// Todo this copy & alloc can be avoided

            let opt_path = qs_parsed_kvs.get("path");
            let opt_id = qs_parsed_kvs.get("id");

            if opt_path.is_none() || opt_id.is_none() {
                return futures::future::ok(responses::bad_request_error("Missing path or id"));
            }


            let body_params = url::form_urlencoded::parse(&bytes_vec[..]).into_owned().collect::<HashMap<String, String>>();
            let str_opt_digits = body_params.get("Digits");
            let mut digits = None;

            if let Some(dig) = str_opt_digits {
                let parse_result = dig.parse::<i32>();

                digits = match parse_result {
                    Err(_) => return futures::future::ok(responses::bad_request_error("Couldn't parse digits")),
                    Ok(parsed) => {
                        if parsed > 9 || parsed < 0 {
                            return futures::future::ok(responses::bad_request_error("Digits should be a single character only!"))
                        }
                        Some(parsed)
                    }
                };
            }


            let (path_str, id_str) = (opt_path.unwrap(), opt_id.unwrap());

            let res_id_i32 = id_str.parse::<i32>();
            if res_id_i32.is_err() {
                return futures::future::ok(responses::bad_request_error("Couldn't parse id"));
            }
            let id_i32 = res_id_i32.unwrap();


            let new_path = format!("{}{}", path_str, digits.map_or("".to_owned(), |x|format!("{}", x)));
            let desired_action = sb_ptr_clone.follow_path(&new_path);

            println!("The desired action is {:?}", desired_action);
            println!("{:?}", *ctx_ptr_clone.borrow());




            let ctx_mgr_ref = ctx_ptr_clone.borrow();

            let this_ctx = ctx_mgr_ref.load_context(id_i32).unwrap(); // TODO UNWRAP


            // These should really all require a hmac
            match desired_action {
                Some((&script::Action::ExecuteScript(ref script), ref new_path)) => {
                    let new_url = format!("{}?path={}&id={}", url_clone, new_path, id_i32);
                    futures::future::ok(hyper::Response::from(twiml::get_input(&new_url, &template(&script.text, this_ctx))))
                }
                Some((&script::Action::HangupWithMessage(ref msg), ref new_path)) => {
                    futures::future::ok(hyper::Response::from(twiml::say(msg)))
                }
                None => futures::future::ok(hyper::Response::from(twiml::say("Invalid path"))),
                _ => panic!("Found some path I don't know how to parse"),
            }
        }));
        result
    }
}


impl TwilioResponseService<ExampleUserContext> {
    fn insert_ctx(&self) -> i32 {

        let ctx = ExampleUserContext {f_name : "will".to_owned(), l_name : "keat".to_owned()};
        self.ctx_ptr.borrow_mut().insert_context(ctx)
    }
}


impl<T> hyper::server::Service for TwilioResponseService<T> where T : ctxmgr::Context + std::fmt::Debug + 'static {
    type Request = hyper::Request;
    type Response = hyper::Response;
    type Error = hyper::Error;
    type Future = Box<futures::Future<Item=Self::Response, Error=Self::Error>>;



    fn call(&self, req: Self::Request) -> Self::Future {
        if req.method() == &hyper::Method::Post {
            return self.handle_twilio(req);
        }


        println!("{:?} {:?}", req.method(), req.path());
        println!("QUERY = {:?}", req.query());
        if req.method() == &hyper::Method::Get && req.path() == "/make_call" {
            let parsed_kvs = url::form_urlencoded::parse(req.query().unwrap_or("").as_bytes()).into_owned().collect::<HashMap<String, String>>();

            {
                let required_keys = ["f_name", "l_name", "phone"];
                let given_keys = parsed_kvs.keys().map(String::as_ref).collect::<Vec<&str>>();
                if !required_keys.iter().all(|k:&&str| given_keys.contains(k)) {
                    return Box::new(futures::future::ok(responses::bad_request_error("Missing some required_keys")))
                }
            }

//            let phone_cp = String::from(parsed_kvs.get("phone"));


            self.ctx_ptr.borrow_mut().insert_context(T::from_kvs(parsed_kvs));

//            let mut evt_loop = tokio_core::reactor::Core::new().unwrap();
//            let twilio_client = twil_api::Twilio::new(&evt_loop.handle());
//
//            twilio_client.start_call()
//
//            println!("{:?}", self.ctx_ptr.borrow());


        }
        Box::new(futures::future::ok(responses::bad_request_error("method/path not supported")))
    }
}


struct ServiceMaker<CTX_T> where CTX_T : ctxmgr::Context {
    ctx_mgr_ptr: std::rc::Rc<std::cell::RefCell<ctxmgr::ContextManager<CTX_T>>>,
    script_base_ptr: std::rc::Rc<script::ScriptBase>,
    pub_url: String
}

impl<CTX_T> ServiceMaker<CTX_T> where CTX_T : ctxmgr::Context {
    fn new(script_base: script::ScriptBase, ctx_mgr: ctxmgr::ContextManager<CTX_T>, url: String) -> ServiceMaker<CTX_T> {
        ServiceMaker {
            pub_url: url,
            script_base_ptr: std::rc::Rc::new(script_base),
            ctx_mgr_ptr: std::rc::Rc::new(std::cell::RefCell::new(ctx_mgr))
        }
    }
}

impl<CTX_T> hyper::server::NewService for ServiceMaker<CTX_T> where CTX_T : ctxmgr::Context + std::fmt::Debug + 'static {
    type Request = <TwilioResponseService<CTX_T> as hyper::server::Service>::Request;
    type Response = <TwilioResponseService<CTX_T> as hyper::server::Service>::Response;
    type Error = <TwilioResponseService<CTX_T> as hyper::server::Service>::Error;
    type Instance = TwilioResponseService<CTX_T>;

    fn new_service(&self) -> Result<Self::Instance, std::io::Error> {
        Ok(TwilioResponseService {pub_url: self.pub_url.clone(), sb_ptr: std::rc::Rc::clone(&self.script_base_ptr), ctx_ptr: std::rc::Rc::clone(&self.ctx_mgr_ptr)})
    }
}

fn launch_ngrok(evt_handle: &tokio_core::reactor::Handle) -> impl Future<Item=String, Error=hyper::error::Error> {
    let ngrok_handle = std::thread::spawn(|| std::process::Command::new("ngrok")
        .args(&["http", "80"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn ngrok, did you install it and put it in the PATH?"));

    let hyper_client = hyper::Client::new(evt_handle);

    let get_ngrok_url_future = hyper_client.get("http://localhost:4040/inspect/http".parse().unwrap())
        .and_then(|hyper_resp| {
            hyper_resp.body().concat2()
        }).map(|resp_body_bytes|{
            let html = String::from_utf8_lossy(&resp_body_bytes);
            let re = regex::Regex::new(r"(https://[a-zA-Z0-9]+\.ngrok\.io)").unwrap();
            let url_match = re.find(&html).expect("Couldn't find url in ngrok dashboard");
            url_match.as_str().to_owned()
        });

    get_ngrok_url_future

}




fn main() {







    let mut evt_loop = tokio_core::reactor::Core::new().unwrap();
    let twilio_client = twil_api::Twilio::new(&evt_loop.handle());

    let handle = &evt_loop.handle();
    let pub_url = evt_loop.run(launch_ngrok(handle)).unwrap();
    println!("Got ngrok url... {}", pub_url);


//    std::process::exit(1);





    use script::{ScriptBase, Script, Action};
    let script_base = ScriptBase::from_root(
        Script::with_text("Hello {f_name}, please press 1 or 2")
            .on(1, Action::ExecuteScript(Script::with_text("You pressed 1, now press 3 or 4")
                .on(3, Action::HangupWithMessage("You pressed 1-3".to_owned()))
                .on(4, Action::HangupWithMessage("You pressed 1-4".to_owned()))
                .on(5, Action::GoToAction("2".to_owned()))
            ))
            .on(2, Action::HangupWithMessage("You pressed 2".to_owned()))
    );

    let mut context_mgr = ctxmgr::ContextManager::<ExampleUserContext>::new();
    context_mgr.insert_context(ExampleUserContext { f_name : "will".to_owned(), l_name : "keat".to_owned()});



    let call_future = twilio_client.start_call("+phone_to_call", &format!("{}?path=&id={}", pub_url, 1));
    //let sjv = evt_loop.run(call_future).unwrap();
//    println!("sjv = {:?}", sjv);

    let ip = "0.0.0.0:80".parse().unwrap();


    let server = hyper::server::Http::new().serve_addr_handle(&ip, &handle,  ServiceMaker::new(script_base, context_mgr, String::from(pub_url))).unwrap();

    println!("Starting server....");


    evt_loop.run(futures::future::empty::<(), ()>()).unwrap();



}
