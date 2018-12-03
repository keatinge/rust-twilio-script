use std::collections::HashMap;



pub trait Context {
    fn resolve_variable<'a>(&'a self, var:&str) -> Option<&'a str>;
    fn list_vars<'a, 'b : 'a>(&'a self) -> &'b[&'b str];
    fn from_kvs(hm:HashMap<String, String>) -> Self;
}


#[derive(Debug)]
pub struct ContextManager<CTX_T> where CTX_T : Context {
    last_call_id: i32,
    contexts: HashMap<i32, CTX_T>
}

impl<CTX_T> ContextManager<CTX_T> where CTX_T: Context + ::std::fmt::Debug {

    pub fn new() -> ContextManager<CTX_T> {
        ContextManager { last_call_id: 0, contexts: HashMap::new() }
    }
    pub fn insert_context(&mut self, context: CTX_T) -> i32 {
        let this_call_id = self.last_call_id + 1;
        let res = self.contexts.insert(this_call_id, context);
        assert!(!res.is_some()); // The id shouldn't already exist in the table
        self.last_call_id = this_call_id;
        this_call_id
    }

    pub fn load_context(&self,  c_id:i32) -> Option<&CTX_T> {
        self.contexts.get(&c_id)
    }
}

