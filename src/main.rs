extern crate toml;
extern crate clap;
extern crate app_dirs;
#[macro_use]
extern crate inner;
extern crate tempdir;
extern crate open;

use toml::Value;
use clap::{Parser, Subcommand, crate_version};
use app_dirs::{AppInfo, AppDataType};
use std::io::{self, Write, Read};
use std::fs::{File, OpenOptions};
use std::{process, env};

const NAME: &'static str = "goal";
const APP_INFO: AppInfo = AppInfo {
    name: NAME,
    author: "pengowen",
};
const VERSION: &'static str = crate_version!();

const GOAL_FILE: &'static str = "goal.toml";
const NO_GOAL_MSG: &'static str = "There is no current goal";

const GOAL_KEY: &'static str = "goal";
const GOAL_TEXT_KEY: &'static str = "text";
const GOAL_DEADLINE_KEY: &'static str = "deadline";

macro_rules! error {
    ($error:expr) => {{
        println!("{}:{} in {}:\n{}", file!(), line!(), module_path!(), $error);
        process::exit(1);
    }}
}

macro_rules! base_goal_file {
    () => {
"[goal]
text = {}
deadline = {}\n"
    }
}

macro_rules! empty_string {
    () => {{
        // 6 quotes; a multiline string in TOML
        r#""""""""#
    }}
}

macro_rules! multiline_string {
    ($e:expr) => {{
        format!(r#""""{}""""#, $e)
    }}
}

fn open_goal_file() -> io::Result<File> {
    let mut path = app_dirs::app_root(AppDataType::UserData, &APP_INFO)
        .or_else(|e| Err(io::Error::new(io::ErrorKind::Other, e)))?;

    path.push(GOAL_FILE);

    if !path.exists() {
        let mut file = File::create(path.clone())?;
        file.write_all(
            format!(
                base_goal_file!(),
                empty_string!(),
                empty_string!()
            ).as_bytes(),
        )?;
    }

    OpenOptions::new().write(true).read(true).open(path)
}

fn parse_goal(text: &str) -> Option<Goal> {
    let parsed: toml::Value = toml::from_str(text).expect(
        "Goal file was invalid: failed to parse",
    );

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

fn set_goal(new_goal: &str, deadline: Option<String>) -> io::Result<()> {
    let mut goal_file = open_goal_file()?;

    goal_file.set_len(0)?;

    let deadline = deadline.map(|d| multiline_string!(d)).unwrap_or(
        empty_string!()
            .to_string(),
    );

    goal_file.write_all(
        format!(
            base_goal_file!(),
            multiline_string!(new_goal),
            deadline
        ).as_bytes(),
    )?;

    println!("Goal set.");

    Ok(())
}

fn edit_goal(editor: Option<String>, new_deadline: Option<String>) -> io::Result<()> {
    let tmp_dir = tempdir::TempDir::new("goal-edit")?;
    let file_path = tmp_dir.path().join("goal-edit");
    let mut tmp_file = File::create(file_path.clone())?;

    // Get current goal (and the deadline for later)
    let current_goal = get_goal()?;
    let (goal, deadline) = current_goal.map(|g| (g.text, g.deadline)).unwrap_or((
        String::new(),
        None,
    ));

    // Write the current goal to the temp file, will be displayed on editor start
    writeln!(tmp_file, "{}", goal)?;

    // If no editor was provided, get it from the EDITOR environment variable
    let editor = editor.unwrap_or_else(|| {
        env::var("EDITOR").unwrap_or_else(|e| {
            println!("Failed to get value of $EDITOR: {}", e);
            process::exit(1);
        })
    });

    // Open the editor
    let exit_code = process::Command::new(editor)
        .arg(file_path.clone())
        .spawn()?
        .wait()?;

    if !exit_code.success() {
        println!("Editor exited with {}, aborting", exit_code);
        process::exit(1);
    }

    // Get the new goal
    let mut tmp_file = File::open(file_path)?;
    let mut new_goal = String::new();
    tmp_file.read_to_string(&mut new_goal)?;

    // Set the goal
    set_goal(new_goal.trim(), new_deadline.or(deadline))?;

    tmp_dir.close()?;

    Ok(())
}

fn remove_goal() -> io::Result<()> {
    let mut goal_file = open_goal_file()?;

    goal_file.set_len(0)?;
    goal_file.write_all(
        format!(base_goal_file!(), empty_string!(), empty_string!()).as_bytes(),
    )
}

fn show_current_goal() -> io::Result<()> {
    let goal = get_goal()?;

    if let Some(goal) = goal {
        println!(
            "Current goal: {}\nDeadline: {}",
            goal.text,
            goal.deadline.unwrap_or("None".to_string())
        );
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
    fn new(text: String, deadline: Option<String>) -> Goal {
        Goal {
            text,
            deadline,
        }
    }
}

/// The top-level clap CLI type
#[derive(Parser)]
#[command(name = NAME)]
#[command(version = VERSION)]
#[command(about = "Keeps track of your current goal")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    /// Shows the current goal
    Show,
    /// Sets the current goal
    Set {
        /// The new goal
        #[arg(value_name = "new goal")]
        new_goal: String,
        /// The deadline for the goal
        #[arg(short, long)]
        deadline: Option<String>,
    },
    /// Opens the goal in your editor
    Edit {
        /// The editor to use
        #[arg(short, long)]
        editor: Option<String>,
        /// The deadline for the goal
        #[arg(short, long)]
        deadline: Option<String>,
    },
    /// Removes the current goal
    Remove,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Show => show_current_goal(),
        Commands::Set { new_goal, deadline } => set_goal(&new_goal, deadline),
        Commands::Edit { editor, deadline } => edit_goal(editor, deadline),
        Commands::Remove => remove_goal(),
    }.unwrap_or_else(|e| error!(e));
}
