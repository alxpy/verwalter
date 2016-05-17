use std::io;
use std::fmt::{Arguments, Debug};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::process::ExitStatus;
use std::collections::HashMap;

use rand::{thread_rng, Rng};
use tempfile::NamedTempFile;
use rustc_serialize::{Decodable, Decoder};
use rustc_serialize::json::{Json, ToJson};

use render;
use apply;
use config::Config;
use render::Error as RenderError;
use watchdog::{ExitOnReturn, Alarm};
use fs_util::write_file;
use apply::expand::Variables;

mod expand;

// commands
pub mod root_command;
pub mod cmd;
pub mod shell;
pub mod copy;
pub mod peek_log;

const COMMANDS: &'static [&'static str] = &[
    "RootCommand",
    "Cmd",
    "Sh",
    "Copy",
    "PeekLog",
];

pub struct Settings {
    pub hostname: String,
    pub dry_run: bool,
    pub log_dir: PathBuf,
    pub schedule_file: PathBuf,
}

pub type ApplyTask = HashMap<String,
    Result<Vec<(String, Command, Source)>, RenderError>>;

pub struct Task<'a: 'b, 'b: 'c, 'c: 'd, 'd> {
    pub runner: &'d str,
    pub log: &'d mut log::Action<'a, 'b, 'c>,
    pub dry_run: bool,
    pub source: &'d Source,
}

trait Action: Debug + Send + ToJson + Sync {
    fn execute(&self, task: Task, variables: Variables)
        -> Result<(), Error>;
}

#[derive(Debug, Clone)]
pub struct Command(Arc<Action>);

pub enum Source {
    TmpFile(NamedTempFile),
}

quick_error!{
    #[derive(Debug)]
    pub enum Error {
        Command(runner: String, cmd: String, status: ExitStatus) {
            display("Action {:?} failed to run {:?}: {}", runner, cmd, status)
            description("error running command")
        }
        CantRun(runner: String, cmd: String, err: io::Error) {
            display("Action {:?} failed to run {:?}: {}", runner, cmd, err)
            description("error running command")
        }
        Log(err: log::Error) {
            from() cause(err)
            display("error opening log file: {}", err)
            description("error logging command info")
        }
        InvalidArgument(message: &'static str, value: String) {
            display("{}: {}", message, value)
            description(message)
        }
        IoError(err: io::Error) {
            from() cause(err)
            display("io error: {}", err)
            description(err.description())
        }
    }
}

fn cmd<T: Action + 'static, E>(val: Result<T, E>)
    -> Result<Command, E>
{
    val.map(|x| Command(Arc::new(x) as Arc<Action>))
}

fn decode_command<D: Decoder>(cmdname: &str, d: &mut D)
    -> Result<Command, D::Error>
{
    match cmdname {
        "RootCommand" => cmd(self::root_command::RootCommand::decode(d)),
        "Cmd" => cmd(self::cmd::Cmd::decode(d)),
        "Sh" => cmd(self::shell::Sh::decode(d)),
        "Copy" => cmd(self::copy::Copy::decode(d)),
        "PeekLog" => cmd(self::peek_log::PeekLog::decode(d)),
        _ => panic!("Command {:?} not implemented", cmdname),
    }
}

impl Decodable for Command {
    fn decode<D: Decoder>(d: &mut D) -> Result<Command, D::Error> {
        Ok(try!(d.read_enum("Action", |d| {
            d.read_enum_variant(&COMMANDS, |d, index| {
                decode_command(COMMANDS[index], d)
            })
        })))
    }
}

impl ToJson for Command {
    fn to_json(&self) -> Json {
        self.0.to_json()
    }
}

impl Action for Command {
    fn execute(&self, task: Task, variables: Variables)
        -> Result<(), Error>
    {
        self.0.execute(task, variables)
    }
}


impl<'a, 'b, 'c, 'd> Task<'a, 'b, 'c, 'd> {
    fn log(&mut self, args: Arguments) {
        if self.dry_run {
            self.log.log(format_args!("(dry_run) {}\n", args));
        } else {
            self.log.log(args);
        }
    }
}

pub fn apply_list(role: &String,
    actions: Vec<(String, Vec<Command>, Source)>,
    log: &mut log::Role, dry_run: bool)
{
    for (aname, commands, source) in actions {
        let mut action = log.action(&aname);
        for cmd in commands {
            let vars = expand::Variables::new()
               .add("role_name", role)
               .add_source(&source);
            let result = cmd.execute(Task {
                runner: &aname,
                log: &mut action,
                dry_run: dry_run,
                source: &source,
            }, vars);
            if let Err(e) = result {
                action.error(&e);
                break;
            }
        }
    }
}

fn apply_schedule(config: &Config, hash: &String, scheduler_result: &Json,
    peers: &HashMap<Id, Peer>, debug: Option<Arc<String>>, settings: &Settings)
{
    let mut rlog = match dlog.role(&role_name) {
        Ok(l) => l,
        Err(e) => {
            panic!("Can't create role log: {}", e);
            return;
        }
    };

}
