use std::process::ExitCode;

use ripenv::main as ripenv_main;

fn main() -> ExitCode {
    ripenv_main(std::env::args_os())
}
