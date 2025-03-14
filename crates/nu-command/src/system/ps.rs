use std::time::Duration;

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoInterruptiblePipelineData, PipelineData, Record, ShellError, Signature,
    Type, Value,
};

#[derive(Clone)]
pub struct Ps;

impl Command for Ps {
    fn name(&self) -> &str {
        "ps"
    }

    fn signature(&self) -> Signature {
        Signature::build("ps")
            .input_output_types(vec![(Type::Nothing, Type::Table(vec![]))])
            .switch(
                "long",
                "list all available columns for each entry",
                Some('l'),
            )
            .filter()
            .category(Category::System)
    }

    fn usage(&self) -> &str {
        "View information about system processes."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["procedures", "operations", "tasks", "ops"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_ps(engine_state, call)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "List the system processes",
                example: "ps",
                result: None,
            },
            Example {
                description: "List the top 5 system processes with the highest memory usage",
                example: "ps | sort-by mem | last 5",
                result: None,
            },
            Example {
                description: "List the top 3 system processes with the highest CPU usage",
                example: "ps | sort-by cpu | last 3",
                result: None,
            },
            Example {
                description: "List the system processes with 'nu' in their names",
                example: "ps | where name =~ 'nu'",
                result: None,
            },
            Example {
                description: "Get the parent process id of the current nu process",
                example: "ps | where pid == $nu.pid | get ppid",
                result: None,
            },
        ]
    }
}

fn run_ps(engine_state: &EngineState, call: &Call) -> Result<PipelineData, ShellError> {
    let mut output = vec![];
    let span = call.head;
    let long = call.has_flag("long");

    for proc in nu_system::collect_proc(Duration::from_millis(100), false) {
        let mut record = Record::new();

        record.push("pid", Value::int(proc.pid() as i64, span));
        record.push("ppid", Value::int(proc.ppid() as i64, span));
        record.push("name", Value::string(proc.name(), span));

        #[cfg(not(windows))]
        {
            // Hide status on Windows until we can find a good way to support it
            record.push("status", Value::string(proc.status(), span));
        }

        record.push("cpu", Value::float(proc.cpu_usage(), span));
        record.push("mem", Value::filesize(proc.mem_size() as i64, span));
        record.push("virtual", Value::filesize(proc.virtual_size() as i64, span));

        if long {
            record.push("command", Value::string(proc.command(), span));
            #[cfg(windows)]
            {
                record.push("cwd", Value::string(proc.cwd(), span));
                record.push(
                    "environment",
                    Value::list(
                        proc.environ()
                            .iter()
                            .map(|x| Value::string(x.to_string(), span))
                            .collect(),
                        span,
                    ),
                );
            }
        }

        output.push(Value::record(record, span));
    }

    Ok(output
        .into_iter()
        .into_pipeline_data(engine_state.ctrlc.clone()))
}
