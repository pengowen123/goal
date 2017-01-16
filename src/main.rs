extern crate toml;
#[macro_use]
extern crate clap;
extern crate app_dirs;
#[macro_use]
extern crate inner;

use toml::Value;
use clap::{App, AppSettings, Arg, SubCommand};
use app_dirs::{AppInfo, AppDataType};
use std::io::{self, Write, Read};
use std::fs::{File, OpenOptions};

const NAME: &'static str = "goal";
const APP_INFO: AppInfo = AppInfo {
    name: NAME,
    author: "pengowen",
};
const VERSION: &'static str = crate_version!();

const GOAL_FILE: &'static str = "goal.toml";
const NO_GOAL_MSG: &'static str = "There is no current goal";

const SET_COMMAND: &'static str = "set";
const REMOVE_COMMAND: &'static str = "remove";
const SHOW_COMMAND: &'static str = "show";

const GOAL_KEY: &'static str = "goal";
const GOAL_TEXT_KEY: &'static str = "text";
const GOAL_DEADLINE_KEY: &'static str = "deadline";

macro_rules! error {
    ($error:expr) => {{
        println!("Error:\n{}", $error);
        return;
    }}
}

macro_rules! base_goal_file {
    () => {
"[goal]
text = {}
deadline = {}\n"
    }
}

fn open_goal_file() -> io::Result<File> {
    let mut path = match app_dirs::app_root(AppDataType::UserData, &APP_INFO) {
        Ok(p) => p,
        Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
    };

    path.push(GOAL_FILE);

    if !path.exists() {
        let mut file = File::create(path.clone())?;
        file.write_all(format!(base_goal_file!(), "\"\"", "\"\"").as_bytes())?;
    }

    OpenOptions::new()
        .write(true)
        .read(true)
        .open(path)
}

fn parse_goal(text: &str) -> Option<Goal> {
    let parsed = toml::Parser::new(text).parse().expect("Goal file was invalid: failed to parse");

    let goal = inner!(
        parsed.get(&GOAL_KEY.to_string())
            .expect("Goal file was invalid: failed to get goal value").clone(),
        if Value::Table, else {
            panic!("Goal file was invalid: goal value was not a table");
        });

    let text = inner!(
        goal.get(&GOAL_TEXT_KEY.to_string())
            .expect("Goal file was invalid: failed to get goal text").clone(),
        if Value::String, else {
            panic!("Goal file was invalid: text value was not a string");
        });


    let deadline = inner!(
        goal.get(&GOAL_DEADLINE_KEY.to_string())
            .expect("Goal file was invalid: failed to get goal deadline").clone(),
        if Value::String, else {
            panic!("Goal file was invalid: deadline value was not a string");
        });

    match (text.is_empty(), deadline.is_empty()) {
        (true, true) => None,
        (false, true) => Some(Goal::new(text, None)),
        _ => Some(Goal::new(text, Some(deadline))),
    }
}

fn get_goal() -> io::Result<Option<Goal>> {
    let mut goal_file = open_goal_file()?;
    let mut goal = String::new();

    goal_file.read_to_string(&mut goal)?;

    Ok(parse_goal(&goal))
}

fn set_goal(new_goal: &str, deadline: Option<&str>) -> io::Result<()> {
    let mut goal_file = open_goal_file()?;

    goal_file.set_len(0)?;

    let deadline = deadline.map(|d| format!("\"{}\"", d)).unwrap_or("\"\"".to_string());

    goal_file.write_all(format!(base_goal_file!(),
                                format!("\"{}\"", new_goal),
                                deadline)
            .as_bytes())?;

    Ok(())
}

fn remove_goal() -> io::Result<()> {
    let mut goal_file = open_goal_file()?;

    goal_file.set_len(0)?;
    goal_file.write_all(format!(base_goal_file!(), "\"\"", "\"\"").as_bytes())
}

fn show_current_goal() -> io::Result<()> {
    let goal = get_goal()?;

    if let Some(goal) = goal {
        println!("Current goal: {}\nDeadline: {}",
                 goal.text,
                 goal.deadline.unwrap_or("None".to_string()));
    } else {
        println!("{}", NO_GOAL_MSG);
    }

    Ok(())
}

struct Goal {
    text: String,
    deadline: Option<String>,
}

impl Goal {
    pub fn new(goal: String, deadline: Option<String>) -> Goal {
        Goal {
            text: goal,
            deadline: deadline,
        }
    }
}

fn main() {
    let matches = App::new(NAME)
        .version(VERSION)
        .about("Keeps track of your current goal")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(SubCommand::with_name(SET_COMMAND)
            .about("Sets the current goal")
            .arg(Arg::with_name("new goal")
                .help("The new goal")
                .required(true))
            .arg(Arg::with_name("deadline")
                .help("The deadline for the goal")
                .short("d")
                .long("deadline")
                .takes_value(true)))
        .subcommand(SubCommand::with_name(REMOVE_COMMAND).about("Removes the current goal"))
        .subcommand(SubCommand::with_name(SHOW_COMMAND).about("Shows the current goal"))
        .get_matches();

    if let Some(matches) = matches.subcommand_matches(SET_COMMAND) {
        let new_goal = matches.value_of("new goal").unwrap();

        if let Err(e) = set_goal(new_goal, matches.value_of("deadline")) {
            error!(e);
        }
    }

    match matches.subcommand_name() {
        Some(SET_COMMAND) => println!("Goal set."),
        Some(REMOVE_COMMAND) => {
            if let Err(e) = remove_goal() {
                error!(e);
            }
        }
        Some(SHOW_COMMAND) => {
            if let Err(e) = show_current_goal() {
                error!(e);
            }
        }
        _ => unreachable!(),
    }
}
