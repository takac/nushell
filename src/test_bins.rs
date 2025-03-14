use nu_cmd_base::hook::{eval_env_change_hook, eval_hook};
use nu_engine::eval_block;
use nu_parser::parse;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::{CliError, PipelineData, Value};
use nu_std::load_standard_library;
use std::io::{self, BufRead, Read, Write};

/// Echo's value of env keys from args
/// Example: nu --testbin env_echo FOO BAR
/// If it it's not present echo's nothing
pub fn echo_env(to_stdout: bool) {
    let args = args();
    for arg in args {
        if let Ok(v) = std::env::var(arg) {
            if to_stdout {
                println!("{v}");
            } else {
                eprintln!("{v}");
            }
        }
    }
}

/// Cross platform echo using println!()
/// Example: nu --testbin echo a b c
/// a b c
pub fn cococo() {
    let args: Vec<String> = args();

    if args.len() > 1 {
        // Write back out all the arguments passed
        // if given at least 1 instead of chickens
        // speaking co co co.
        println!("{}", &args[1..].join(" "));
    } else {
        println!("cococo");
    }
}

/// Cross platform cat (open a file, print the contents) using read_to_string and println!()
pub fn meow() {
    let args: Vec<String> = args();

    for arg in args.iter().skip(1) {
        let contents = std::fs::read_to_string(arg).expect("Expected a filepath");
        println!("{contents}");
    }
}

/// Cross platform cat (open a file, print the contents) using read() and write_all() / binary
pub fn meowb() {
    let args: Vec<String> = args();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for arg in args.iter().skip(1) {
        let buf = std::fs::read(arg).expect("Expected a filepath");
        handle.write_all(&buf).expect("failed to write to stdout");
    }
}

// Relays anything received on stdin to stdout
pub fn relay() {
    io::copy(&mut io::stdin().lock(), &mut io::stdout().lock())
        .expect("failed to copy stdin to stdout");
}

/// Cross platform echo but concats arguments without space and NO newline
/// nu --testbin nonu a b c
/// abc
pub fn nonu() {
    args().iter().skip(1).for_each(|arg| print!("{arg}"));
}

/// Repeat a string or char N times
/// nu --testbin repeater a 5
/// aaaaa
/// nu --testbin repeater test 5
/// testtesttesttesttest
pub fn repeater() {
    let mut stdout = io::stdout();
    let args = args();
    let mut args = args.iter().skip(1);
    let letter = args.next().expect("needs a character to iterate");
    let count = args.next().expect("need the number of times to iterate");

    let count: u64 = count.parse().expect("can't convert count to number");

    for _ in 0..count {
        let _ = write!(stdout, "{letter}");
    }
    let _ = stdout.flush();
}

/// A version of repeater that can output binary data, even null bytes
pub fn repeat_bytes() {
    let mut stdout = io::stdout();
    let args = args();
    let mut args = args.iter().skip(1);

    while let (Some(binary), Some(count)) = (args.next(), args.next()) {
        let bytes: Vec<u8> = (0..binary.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&binary[i..i + 2], 16)
                    .expect("binary string is valid hexadecimal")
            })
            .collect();
        let count: u64 = count.parse().expect("repeat count must be a number");

        for _ in 0..count {
            stdout
                .write_all(&bytes)
                .expect("writing to stdout must not fail");
        }
    }

    let _ = stdout.flush();
}

/// Another type of echo that outputs a parameter per line, looping infinitely
pub fn iecho() {
    // println! panics if stdout gets closed, whereas writeln gives us an error
    let mut stdout = io::stdout();
    let _ = args()
        .iter()
        .skip(1)
        .cycle()
        .try_for_each(|v| writeln!(stdout, "{v}"));
}

pub fn fail() {
    std::process::exit(1);
}

/// With no parameters, will chop a character off the end of each line
pub fn chop() {
    if did_chop_arguments() {
        // we are done and don't care about standard input.
        std::process::exit(0);
    }

    // if no arguments given, chop from standard input and exit.
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for given in stdin.lock().lines().flatten() {
        let chopped = if given.is_empty() {
            &given
        } else {
            let to = given.len() - 1;
            &given[..to]
        };

        if let Err(_e) = writeln!(stdout, "{chopped}") {
            break;
        }
    }

    std::process::exit(0);
}

fn outcome_err(
    engine_state: &EngineState,
    error: &(dyn miette::Diagnostic + Send + Sync + 'static),
) -> ! {
    let working_set = StateWorkingSet::new(engine_state);

    eprintln!("Error: {:?}", CliError(error, &working_set));

    std::process::exit(1);
}

fn outcome_ok(msg: String) -> ! {
    println!("{msg}");

    std::process::exit(0);
}

/// Generate a minimal engine state with just `nu-cmd-lang`, `nu-command`, and `nu-cli` commands.
fn get_engine_state() -> EngineState {
    let engine_state = nu_cmd_lang::create_default_context();
    let engine_state = nu_command::add_shell_command_context(engine_state);
    nu_cli::add_cli_context(engine_state)
}

pub fn nu_repl() {
    //cwd: &str, source_lines: &[&str]) {
    let cwd = std::env::current_dir().expect("Could not get current working directory.");
    let source_lines = args();

    let mut engine_state = get_engine_state();
    let mut stack = Stack::new();

    engine_state.add_env_var("PWD".into(), Value::test_string(cwd.to_string_lossy()));

    let mut last_output = String::new();

    load_standard_library(&mut engine_state).expect("Could not load the standard library.");

    for (i, line) in source_lines.iter().enumerate() {
        let cwd = nu_engine::env::current_dir(&engine_state, &stack)
            .unwrap_or_else(|err| outcome_err(&engine_state, &err));

        // Before doing anything, merge the environment from the previous REPL iteration into the
        // permanent state.
        if let Err(err) = engine_state.merge_env(&mut stack, &cwd) {
            outcome_err(&engine_state, &err);
        }

        // Check for pre_prompt hook
        let config = engine_state.get_config();
        if let Some(hook) = config.hooks.pre_prompt.clone() {
            if let Err(err) = eval_hook(
                &mut engine_state,
                &mut stack,
                None,
                vec![],
                &hook,
                "pre_prompt",
            ) {
                outcome_err(&engine_state, &err);
            }
        }

        // Check for env change hook
        let config = engine_state.get_config();
        if let Err(err) = eval_env_change_hook(
            config.hooks.env_change.clone(),
            &mut engine_state,
            &mut stack,
        ) {
            outcome_err(&engine_state, &err);
        }

        // Check for pre_execution hook
        let config = engine_state.get_config();

        engine_state
            .repl_state
            .lock()
            .expect("repl state mutex")
            .buffer = line.to_string();

        if let Some(hook) = config.hooks.pre_execution.clone() {
            if let Err(err) = eval_hook(
                &mut engine_state,
                &mut stack,
                None,
                vec![],
                &hook,
                "pre_execution",
            ) {
                outcome_err(&engine_state, &err);
            }
        }

        // Eval the REPL line
        let (block, delta) = {
            let mut working_set = StateWorkingSet::new(&engine_state);
            let block = parse(
                &mut working_set,
                Some(&format!("line{i}")),
                line.as_bytes(),
                false,
            );

            if let Some(err) = working_set.parse_errors.first() {
                outcome_err(&engine_state, err);
            }
            (block, working_set.render())
        };

        if let Err(err) = engine_state.merge_delta(delta) {
            outcome_err(&engine_state, &err);
        }

        let input = PipelineData::empty();
        let config = engine_state.get_config();

        match eval_block(&engine_state, &mut stack, &block, input, false, false) {
            Ok(pipeline_data) => match pipeline_data.collect_string("", config) {
                Ok(s) => last_output = s,
                Err(err) => outcome_err(&engine_state, &err),
            },
            Err(err) => outcome_err(&engine_state, &err),
        }

        if let Some(cwd) = stack.get_env_var(&engine_state, "PWD") {
            let path = cwd
                .as_string()
                .unwrap_or_else(|err| outcome_err(&engine_state, &err));
            let _ = std::env::set_current_dir(path);
            engine_state.add_env_var("PWD".into(), cwd);
        }
    }

    outcome_ok(last_output)
}

fn did_chop_arguments() -> bool {
    let args: Vec<String> = args();

    if args.len() > 1 {
        let mut arguments = args.iter();
        arguments.next();

        for arg in arguments {
            let chopped = if arg.is_empty() {
                arg
            } else {
                let to = arg.len() - 1;
                &arg[..to]
            };

            println!("{chopped}");
        }

        return true;
    }

    false
}

pub fn input_bytes_length() {
    let stdin = io::stdin();
    let count = stdin.lock().bytes().count();

    println!("{}", count);
}

fn args() -> Vec<String> {
    // skip (--testbin bin_name args)
    std::env::args().skip(2).collect()
}
