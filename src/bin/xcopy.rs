// File: src\bin\xcopy.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Date: 2026-01-24
// Description: copyx - Windows file copy utility using shell operations 
// License: MIT

use std::env;
use std::path::Path;
use xcom::{logs, process_sources, FileOperation};
use clap_version_flag::colorful_version;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && (args[1] == "-v" || args[1] == "--version") {
        let version = colorful_version!();
        version.print_and_exit();
    }

    if args.len() < 3 {
        eprintln!("USAGE: {} SOURCE1 [SOURCE2 ...] DESTINATION", args[0]);
        std::process::exit(1);
    }

    let sources = args[1..args.len() - 1].to_vec();
    let dest = Path::new(&args[args.len() - 1]);

    match process_sources(sources, dest, FileOperation::Copy) {
        Ok(_) => {
            // Operation completed successfully
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            logs(&format!("ERROR: {}", e));
            std::process::exit(1);
        }
    }
}