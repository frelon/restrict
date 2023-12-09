#[macro_use]
extern crate clap;

#[macro_use(defer)]
extern crate scopeguard;

use std::{
    str::FromStr,
    process::{exit, id},
    env,
};

use restrict::Restrict;
use clap::ArgMatches;
use bytesize::ByteSize;

pub mod restrict;

fn main() {
    let matches = clap_app!(Restrict =>
        (version: "0.0.1")
        (author: "Fredrik Lönnegren <fredrik@frelon.se>")
        (about: "Restrict a commands resource usage")
        (@arg DEBUG: -d --debug "Print debug information")
        (@arg CGROUP_PATH: --group ... "cgroup path, can be used to join the same cgroup for multiple invocations")
        (@arg SHELL: -s ... "Set the shell to use for invoking command (defaults to $SHELL)")
        (@arg MEMORY_LIMIT: -m --memory ... "Set memory limit for the command (For example 100M)")
        (@arg CPU_SHARES: -c --cpu ... "Set CPU shares for the command")
        (@arg COMMAND: +last +required "The command to place restrictions on")
    ).get_matches();

    let restrict = restrict_from_cli(matches);

    match restrict.run() {
        Ok(status) => exit(status.code().unwrap()),
        Err(err) => eprintln!("Error: {}", err),
    }
}

fn restrict_from_cli(matches:ArgMatches) -> Restrict {
    let shell = matches.value_of("SHELL")
        .unwrap_or(&*env::var("SHELL").expect("SHELL not provided and $SHELL not set"))
        .to_string();

    let command = matches.value_of("COMMAND")
        .expect("COMMAND required")
        .to_string();

    let debug = matches.is_present("DEBUG");

    let mem = matches.value_of("MEMORY_LIMIT")
        .as_ref()
        .map(|size| 
             ByteSize::from_str(size)
             .unwrap()
        );

    let cpu = matches.value_of("CPU_SHARES")
        .map(|size| 
             u64::from_str(size).expect("must be an unsigned integer")
        );

    let cgroup_path = matches.value_of("CGROUP_PATH")
        .map(|path| path.to_string())
        .unwrap_or(format!("restrict-{}", id()));

    Restrict::new()
        .with_shell(shell)
        .with_command(command)
        .with_debug(debug)
        .with_memory_limit(mem)
        .with_cpu_limit(cpu)
        .with_cgroup_path(cgroup_path)
}

