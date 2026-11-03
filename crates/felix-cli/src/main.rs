//! `felix` — parse, check, run; draw, seal, verify.
//!
//! Exit codes: `0` success, `1` the program or capsule was refused (a
//! diagnostic is on stderr), `2` the command line itself was wrong.

use std::io::Read;
use std::process::ExitCode;

use felix_lang::{capsule, compile, graph, run, Diagnostic};

const USAGE: &str = "\
felix v1.0 — linear values, exact p/q scales, and a ternary datapath pinned to the memnet oracle

usage:
  felix <file.flx>                    run a program
  felix run <file.flx | ->            run a program (`-` reads stdin)
  felix -e '<source>'                 run inline source, one shot
  felix check <file.flx | ->          parse and type-check only
  felix graph [--mermaid] <file.flx>  print the dataflow graph (DOT, or Mermaid)
  felix capsule <file.flx | ->        seal the program and its transcript
  felix verify <capsule | ->          re-run a capsule; fail unless it reproduces
  felix --version | --help";

enum Fail {
    /// The command line was wrong: exit 2.
    Usage(String),
    /// The input was refused: exit 1. Already printed.
    Refused,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match dispatch(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Fail::Refused) => ExitCode::from(1),
        Err(Fail::Usage(message)) => {
            eprintln!("felix: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn dispatch(args: &[String]) -> Result<(), Fail> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] => Err(Fail::Usage("no input".into())),
        ["--help" | "-h"] => {
            println!("{USAGE}");
            Ok(())
        }
        ["--version" | "-V"] => {
            println!("felix {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        ["-e", source] => cmd_run(source),
        ["-e"] => Err(Fail::Usage("`-e` needs the source as its argument".into())),
        ["run", path] => cmd_run(&read(path)?),
        ["check", path] => cmd_check(&read(path)?),
        ["graph", "--mermaid"] => Err(Fail::Usage("`graph --mermaid` needs a file".into())),
        ["graph", path] => cmd_graph(&read(path)?, false),
        ["graph", "--mermaid", path] | ["graph", path, "--mermaid"] => {
            cmd_graph(&read(path)?, true)
        }
        ["capsule", path] => cmd_capsule(&read(path)?),
        ["verify", path] => cmd_verify(&read(path)?),
        ["-"] => cmd_run(&read("-")?),
        [path] if !path.starts_with('-') && !is_command(path) => cmd_run(&read(path)?),
        [command, ..] if is_command(command) => {
            Err(Fail::Usage(format!("`{command}` takes exactly one input")))
        }
        [flag, ..] if flag.starts_with('-') => Err(Fail::Usage(format!("unknown flag `{flag}`"))),
        _ => Err(Fail::Usage("felix takes one input at a time".into())),
    }
}

fn is_command(word: &str) -> bool {
    matches!(word, "run" | "check" | "graph" | "capsule" | "verify")
}

fn read(path: &str) -> Result<String, Fail> {
    let result = if path == "-" {
        let mut text = String::new();
        std::io::stdin().read_to_string(&mut text).map(|_| text)
    } else {
        std::fs::read_to_string(path)
    };
    result.map_err(|error| {
        eprintln!("felix: cannot read {path}: {error}");
        Fail::Refused
    })
}

fn refuse(source: &str, problems: &[Diagnostic]) -> Fail {
    for problem in problems {
        eprintln!("{}", problem.render(source));
    }
    Fail::Refused
}

fn cmd_run(source: &str) -> Result<(), Fail> {
    let outputs = run(source).map_err(|p| refuse(source, &p))?;
    for (rendered, _) in outputs {
        println!("{rendered}");
    }
    Ok(())
}

fn cmd_check(source: &str) -> Result<(), Fail> {
    let (program, checked) = compile(source).map_err(|p| refuse(source, &p))?;
    let linear = checked.bindings.iter().filter(|b| b.linear).count();
    println!(
        "ok: {} statements, {} bindings, {linear} linear",
        program.statements.len(),
        checked.bindings.len()
    );
    Ok(())
}

fn cmd_graph(source: &str, mermaid: bool) -> Result<(), Fail> {
    let (program, checked) = compile(source).map_err(|p| refuse(source, &p))?;
    let g = graph::build(&program, &checked);
    print!("{}", if mermaid { g.to_mermaid() } else { g.to_dot() });
    Ok(())
}

fn cmd_capsule(source: &str) -> Result<(), Fail> {
    let sealed = capsule::seal(source).map_err(|p| refuse(source, &p))?;
    print!("{sealed}");
    Ok(())
}

fn cmd_verify(text: &str) -> Result<(), Fail> {
    match capsule::verify(text) {
        Ok(c) => {
            println!(
                "ok: capsule {} reproduces {} output lines",
                c.id,
                c.output.lines().count()
            );
            Ok(())
        }
        Err(error) => {
            eprintln!("felix: {error}");
            Err(Fail::Refused)
        }
    }
}
