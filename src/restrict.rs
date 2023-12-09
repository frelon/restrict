use std::{ 
    process::{Child, Command, exit, id, Stdio, ExitStatus},
    os::unix::process::CommandExt,
};

use colored::*;
use bytesize::ByteSize;

use cgroups_rs::{
    {Cgroup, MaxValue, CgroupPid},
    memory::{MemController, SetMemory},
    cpu::CpuController,
};

pub struct Restrict {
    debug: bool,
    memory_limit: Option<ByteSize>,
    cpu_shares: Option<u64>,
    shell: String,
    command: String,
    cgroup_path: String,
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
            cgroup_path: "restrict".to_string(),
        }
    }

    gen!(with_shell,shell,String);
    gen!(with_command,command,String);
    gen!(with_debug,debug,bool);
    gen!(with_memory_limit,memory_limit,Option<ByteSize>);
    gen!(with_cpu_limit,cpu_shares,Option<u64>);
    gen!(with_cgroup_path,cgroup_path,String);

    pub fn run(self) -> Result<ExitStatus, String> {
        let h = cgroups_rs::hierarchies::auto();
        if !h.v2() {
           eprintln!("needs cgroups v2");
           return Err("needs cgroups v2".to_string()); 
        }

        let cg = Cgroup::new(h, self.cgroup_path.clone()).unwrap();
        let cg2 = cg.clone();
        defer! {
            if let Err(err) = cg2.delete() {
                eprintln!("failed deleting cgroup: {}", err.to_string().red());
            }
        }

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
    
        let mut child:Child;

        unsafe {
            child = Command::new(self.shell)
                .stdout(Stdio::inherit())
                .stdin(Stdio::inherit())
                .stderr(Stdio::inherit())
                .arg("-c")
                .arg(self.command)
                .pre_exec(move || {
                    Self::add_pid_to_cgroup(&cg, id().into());
                    Ok(())
                })
                .spawn()
                .expect("process failed");
        }

        match child.wait() {
            Ok(status) => {
                if self.debug {
                    if status.success() {
                        println!("command exited with status {}", status);
                    } else {
                        eprintln!("{}", format!("{}", status).red());
                    }
                }

                Ok(status)
            },
            Err(err) => Err(err.to_string())
        }
    }

    fn print_restrict_info(&self) {
        println!("run command {}", format!("{} -c '{}'", self.shell, self.command.yellow()).bold());
        println!("\tcgroup {}", self.cgroup_path.green());

        match self.cpu_shares {
            Some(cpu) => println!("\tcpu    {} {} shares", "restricted to ".green(), cpu),
            None =>      println!("\tcpu    {}", "unrestricted".red()),
        }

        match self.memory_limit {
            Some(mem) => println!("\tmemory {} {}", "restricted to ".green(), mem),
            None =>      println!("\tmemory {}", "unrestricted".red()),
        }
    }

    fn add_pid_to_cgroup(cgroup:&Cgroup, pid:u64) {
        match cgroup.add_task(CgroupPid::from(pid)) {
            Ok(()) => {},
            Err(err) => eprintln!("failed adding task to cgroup: {}", err),
        }
    }
}
