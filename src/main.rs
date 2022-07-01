use clap::Parser;
use std::sync::Arc;
use std::process::{Command, exit, id};
use std::os::unix::process::CommandExt;
use std::io::{self, Write};
use parse_size::parse_size;
use cgroups_rs::*;
use cgroups_rs::memory::{MemController, SetMemory};
use cgroups_rs::cpu::{CpuController};
use cgroups_rs::{Cgroup, MaxValue};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(last = true, value_parser)]
    command: Vec<String>,

    #[clap(short='m', long, value_parser)]
    memory_limit: Option<String>,

    #[clap(short='c', long, value_parser)]
    cpu_limit: Option<u64>,

    #[clap(short='d', long, value_parser)]
    debug: bool,
}

fn main() {
    let args = Args::parse();
    let debug: Arc<bool> = Arc::new(args.debug);

    let mem = match args.memory_limit {
        Some(ref size) => Some(parse_size(size).unwrap()),
        None => None,
    };

    if *debug {
        println!("parent {} running command '{}'", id(), args.command.join(" "));

        match mem {
            Some(_) => println!("\trestricted to {} memory", args.memory_limit.clone().unwrap()),
            None => println!("\tunrestricted memory"),
        };

        match args.cpu_limit {
            Some(cpu) => println!("\trestricted to {}% cpu", cpu),
            None => println!("\tunrestricted cpu"),
        };
    }

    let h = cgroups_rs::hierarchies::auto();
    if !h.v2() {
        eprintln!("needs cgroups v2");
        return;
    }

    let cg = Cgroup::new(h, "restrict_me_plox");

    if mem.is_some() {
        let mem_controller: &MemController = cg.controller_of().unwrap();

        let m = SetMemory {
                max: Some(MaxValue::Value(mem.unwrap() as i64)),
                min: Some(MaxValue::Value(1)),
                high: Some(MaxValue::Max),
                low: None,
            };
        let r = mem_controller.set_mem(m);

        match r {
            Err(err) => {
                eprintln!("failed setting cgroup memory: {}", err);
                exit(1);
            },
            _ => {},
        }
    }

    if args.cpu_limit.is_some() {
        let cpu_controller: &CpuController = cg.controller_of().unwrap();

        let r = cpu_controller.set_shares(args.cpu_limit.unwrap());

        match r {
            Err(err) => {
                eprintln!("failed setting cgroup cpu: {}", err);
                exit(1);
            },
            _ => {},
        }
    }

    unsafe {
        let result = Command::new(args.command.clone().into_iter().nth(0).unwrap())
            .args(args.command.into_iter().skip(1).collect::<Vec<String>>())
            .pre_exec(move || {
                add_pid_to_cgroup(&cg, id());
                Ok(())
            })
            .output();

        match result {
            Ok(output) => {
                io::stdout().write_all(&output.stdout).unwrap();
                io::stderr().write_all(&output.stderr).unwrap();

                if let Some(status) = output.status.code() {
                    if args.debug {
                        println!("command exited with status {}", status);
                    }

                    exit(status);
                }
            }
            Err(err) => {
                eprintln!("failed executing command: {}", err);
            },
        }
    }
}

fn add_pid_to_cgroup(cgroup:&Cgroup, pid:u32) {
    match cgroup.add_task(CgroupPid::from(pid as u64)) {
        Ok(()) => {},
        Err(err) => eprintln!("failed adding task to cgroup: {}", err),
    }
}
