#[derive(Debug)]

pub enum Action {
    ExecuteScript(Script),
    HangupWithMessage(String),
    GoToAction(String),
    Repeat,
}


#[derive(Debug)]
pub struct ScriptBase {
    pub root: Action
}

impl ScriptBase {

    pub fn from_root(root: Script) -> ScriptBase {

        ScriptBase { root: Action::ExecuteScript(root) }
    }

    /// Returns the action at that path and the new path, new path exists because
    /// if you made an error and your err is set to Action::Repeat it will have
    /// to adjust your path
    pub fn follow_path(&self, path: &str) -> Option<(&Action, String)> {
        let mut cur: &Action = &self.root;
        for c in path.chars() {
            let dig_opt = c.to_digit(10);

            if dig_opt.is_none() { return None }

            let digit = dig_opt.unwrap();


            match cur {
                 &Action::ExecuteScript(ref script_ref) => {
                     let act_opt = &script_ref.other_scripts[digit as usize];
                     if act_opt.is_none() {

                         match *script_ref.err {
                             Action::Repeat => {
                                 // When you take a wrong turn down a path, and hit the action Action::Repeat
                                 // you want to remove the last digit from the path so that path no longer
                                 // contains the invalid turn
                                 return Some((cur, path.clone().chars().into_iter().take(path.len()-1).collect::<String>()))
                             },
                             _=> return Some((&script_ref.err, String::from(path)))
                         }

                     }
                     cur = act_opt.as_ref().unwrap();

                }
                _=> {
                    println!(" -- Path {:?} makes no sense, attempted to go down a path where there is none -- ", path);
                    return None
                }
            }
        }

        if let &Action::GoToAction(ref new_path) = cur {
            self.follow_path(new_path)
        }
        else {
            Some((cur, String::from(path)))
        }
    }
}

#[derive(Debug)]

pub struct Script {
    pub text: String,
    err: Box<Action>,
    other_scripts: Vec<Option<Action>>
}


impl Script {

    pub fn with_text(s: &str) -> Script {
        let default_err_option = Box::new(Action::Repeat);
        Script { text: String::from(s), other_scripts: (0..10).map(|_| None ).collect() , err: default_err_option}
    }

    pub fn on(mut self, key: usize, act: Action) -> Self {
        self.other_scripts[key] = Some(act);
        self
    }


}
























































