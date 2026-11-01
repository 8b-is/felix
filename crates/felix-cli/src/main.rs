//! `felix` — parse, check, run. One file in, values out, or a diagnostic that
//! points at the line and column.

use std::process::ExitCode;

use felix_lang::{check, eval, parser};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: felix <file.flx>");
        return ExitCode::FAILURE;
    };
    if let Some(extra) = args.next() {
        eprintln!("felix takes one file at a time; ignoring `{extra}`");
    }

    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("felix: cannot read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };

    let program = match parser::parse(&source) {
        Ok(program) => program,
        Err(diagnostic) => {
            eprintln!("{}", diagnostic.render(&source));
            return ExitCode::FAILURE;
        }
    };

    if let Err(problems) = check::check(&program) {
        for problem in problems {
            eprintln!("{}", problem.render(&source));
        }
        return ExitCode::FAILURE;
    }

    for (rendered, _) in eval::run(&program) {
        println!("{rendered}");
    }
    ExitCode::SUCCESS
}
