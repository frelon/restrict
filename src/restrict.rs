use std::process::{Command, exit, id};
use std::os::unix::process::CommandExt;
use std::io::{self, Write};

use colored::*;
use bytesize::ByteSize;

use cgroups_rs::*;
use cgroups_rs::memory::{MemController, SetMemory};
use cgroups_rs::cpu::CpuController;
use cgroups_rs::{Cgroup, MaxValue};

pub struct Restrict {
    debug: bool,
    memory_limit: Option<ByteSize>,
    cpu_shares: Option<u64>,
    shell: String,
    command: String,
}

macro_rules! gen {
    ($fn_name:ident, $name:ident, $t:ty) => { 
        pub fn $fn_name(mut self, $name:$t) -> Self {
            self.$name = $name;

            self
        }
    };
}

impl Default for Restrict {
    fn default() -> Self {
        Self::new()
    }
}

impl Restrict {
    pub fn new() -> Self {
        Restrict{
            debug: false,
            memory_limit: None,
            cpu_shares: None,
            shell: "".to_string(),
            command: "".to_string(),
        }
    }

    gen!(with_shell,shell,String);
    gen!(with_command,command,String);
    gen!(with_debug,debug,bool);
    gen!(with_memory_limit,memory_limit,Option<ByteSize>);
    gen!(with_cpu_limit,cpu_shares,Option<u64>);

    pub fn run(self) -> Result<i32, String> {
        let h = cgroups_rs::hierarchies::auto();
        if !h.v2() {
            eprintln!("needs cgroups v2");
            return Err("needs cgroups v2".to_string()); 
        }

        let cg = Cgroup::new(h, "restrict");

        if let Some(memory_limit) = self.memory_limit {
            let mem_controller: &MemController = cg.controller_of().unwrap();

            let m = SetMemory {
                max: Some(MaxValue::Value(memory_limit.as_u64().try_into().unwrap())),
                min: Some(MaxValue::Value(1)),
                high: Some(MaxValue::Max),
                low: None,
            };

            if let Err(err) = mem_controller.set_mem(m) {
                eprintln!("failed setting cgroup memory: {}", err);
                exit(1);
            }
        }

        if let Some(cpu_limit) = self.cpu_shares {
            let cpu_controller: &CpuController = cg.controller_of().unwrap();

            if let Err(err) = cpu_controller.set_shares(cpu_limit) {
                eprintln!("failed setting cgroup cpu: {}", err);
                exit(1);
            }
        }

        if self.debug {
            self.print_restrict_info();
        }


        unsafe {
            let result = Command::new(self.shell)
                .arg("-c")
                .arg(self.command)
                .pre_exec(move || {
                    Self::add_pid_to_cgroup(&cg, id());
                    Ok(())
                })
                .output();

            match result {
                Ok(output) => {
                    io::stdout().write_all(&output.stdout).unwrap();
                    io::stderr().write_all(&output.stderr).unwrap();

                    if let Some(status) = output.status.code() {
                        if self.debug {
                            println!("command exited with status {}", status);
                        }

                        return Ok(status);
                    }
                }
                Err(err) => {
                    eprintln!("failed executing command: {}", err);
                    return Err(err.to_string())
                },
            }
        }

        unreachable!()
    }

    fn print_restrict_info(&self) {
        println!("run command {}", format!("{} -c '{}'", self.shell, self.command.yellow()).bold());

        match self.cpu_shares {
            Some(cpu) => println!("\tcpu    {} {} shares", "restricted to ".green(), cpu),
            None =>      println!("\tcpu    {}", "unrestricted".red()),
        }

        match self.memory_limit {
            Some(mem) => println!("\tmemory {} {}", "restricted to ".green(), mem),
            None =>      println!("\tmemory {}", "unrestricted".red()),
        }
    }

    fn add_pid_to_cgroup(cgroup:&Cgroup, pid:u32) {
        match cgroup.add_task(CgroupPid::from(pid as u64)) {
            Ok(()) => {},
            Err(err) => eprintln!("failed adding task to cgroup: {}", err),
        }
    }
}
