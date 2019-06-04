extern crate libc;

use std::io;
use std::io::prelude::*;
use std::process::Stdio;
use std::fs::File;

struct User {
    inn: Option<Stdio>,
    out: Option<Stdio>,
    builtin_pipe: Option<String>,
    cmd: String,
    bg: bool,
    args: Vec<String>,
    next_command: Option<Box<User>>,
}

impl User {
    pub fn new(line: &str) -> Self {
        let mut tokens = line.split_whitespace();
        let cmd = tokens.next().unwrap().to_owned();
        let mut user = User {
            cmd: cmd,
            inn: None,
            out: None,
            builtin_pipe: None,
            bg: false,
            args: Vec::new(),
            next_command: None,
        };

        while let Some(token) = tokens.next() {
            match token {
                "|" => {
                    let next = tokens.clone().fold(String::new(), |acc, s| acc + " " + s);
                    user.next_command = Some(Box::new(User::new(&next)));
                    break;
                }
                "&" => user.bg = true,
                "<" => {
                    let filename = tokens.next().expect("filename is required after <");
                    let file = File::open(filename).expect("Error opening file");
                    user.inn = Some(file.into());
                }
                ">" => {
                    let filename = tokens.next().expect("filename is required after >");
                    let file = File::create(filename).expect("Error outputting");
                    user.out = Some(file.into());
                }
                arg => {
                    user.args.push(arg.to_owned());
                }
            }
        }

        user
    }

    pub fn run(&mut self, history: &[String]) {
        match &*self.cmd {
            "cd" => {
                if let Some(ref mut new_command) = self.next_command {
                    new_command.run(&history);
                }else {
                    std::env::set_current_dir(&self.args[0]).unwrap();
                }

            }
            "exit" => {
                 if let Some(ref mut new_command) = self.next_command {
                    new_command.run(&history);
                }else {
                    std::process::exit(0);
                }
            },
            "history" => {
                if let Some(ref mut new_command) = self.next_command {
                    let mut tmp: String = String::new();
                    for (i, command) in history.iter().enumerate() {
                            tmp.push_str("    ");
                            tmp.push_str(&(i+1).to_string());
                            tmp.push_str("  ");
                            tmp.push_str(&history[i].clone());
                            tmp.push('\n');
                    }
                    new_command.builtin_pipe = Some(tmp);
                    new_command.run(&history);
                }else {
                    for (i, command) in history.iter().enumerate() {
                        println!("    {}  {}", i + 1, command);
                    }
                }
            },

            "jobs" => {},

            "kill" => unsafe {
                if let Some(ref mut new_command) = self.next_command {
                        new_command.run(&history);
                    }else {
                        libc::kill(self.args[0].parse::<i32>().unwrap(), libc::SIGTERM);
                }
            },

            "pwd" => {
                if let Some(ref mut new_command) = self.next_command {
                    let mut tmp: String =  std::env::current_dir().unwrap().display().to_string();
                    tmp.push('\n');
                    new_command.builtin_pipe = Some(tmp);
                    new_command.run(&history);
                }else {
                    println!("{}", std::env::current_dir().unwrap().display());
                }

            }

            command => {
                use std::process::Command;

                let mut command = Command::new(command);
                command.args(&self.args);

                if let Some(file) = self.inn.take() {
                    command.stdin(file);
                }

                if let Some(file) = self.out.take() {
                    command.stdout(file);
                }
               // println!("hello2");

                if let Some(ref mut new_command) = self.next_command {
                    if let Some(ref file) = self.builtin_pipe {
                        let mut output = command.stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
                        output.stdin.as_mut().unwrap().write_all(file.as_bytes());
                        if !&self.bg {
                         output.wait().unwrap();
                        }
                        new_command.inn = Some(output.stdout.unwrap().into());
                        //println!("hello");
                        new_command.run(&history);

                    }else {
                      //  println!("hello2");
                      let mut output = command.stdout(Stdio::piped()).spawn().unwrap();
                       if !&self.bg {
                         output.wait().unwrap();
                        }
                        new_command.inn = Some(output.stdout.unwrap().into());
                        new_command.run(&history);
                    }
                }else if let Some(ref file) = self.builtin_pipe {

                    let mut output = command.stdin(Stdio::piped()).spawn().unwrap();
                    output.stdin.as_mut().unwrap().write_all(file.as_bytes());
                    if !&self.bg {
                        output.wait().unwrap();
                    }
                }else {
              // println!("hello2");
                    let mut output = command.spawn().unwrap();
                    if !&self.bg {
                        output.wait().unwrap();
                    }
                }
            }
        }
    }
}

fn main() {
    let stdin = io::stdin();
    let mut history = Vec::new();
    print!("$ ");
    io::stdout().flush().unwrap();
    for line in stdin.lock().lines() {
        match line {
            Ok(line) => {
                let line = line.trim();
                let mut user = User::new(&line);
                user.run(&history);
                history.push(line.to_string());
            }
            Err(_) => break,
        }
        print!("$ ");
        io::stdout().flush().unwrap();
    }
}
