use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Type, Value,
};
use rand::prelude::SliceRandom;
use rand::thread_rng;

#[derive(Clone)]
pub struct Shuffle;

impl Command for Shuffle {
    fn name(&self) -> &str {
        "shuffle"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("shuffle")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::Record(vec![]), Type::Record(vec![])),
            ])
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Shuffle rows randomly."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let metadata = input.metadata();
        let span = input.span().unwrap_or(call.head);

        match input {
            // Records have two sorting methods, toggled by presence or absence of -v
            PipelineData::Value(Value::Record { val, .. }, ..) => {
                let mut input_pairs: Vec<(String, Value)> = val.into_iter().collect();
                let mut thread_rng = rand::thread_rng();
                input_pairs.shuffle(&mut thread_rng);
                let record = Value::record(input_pairs.into_iter().collect(), span);
                Ok(record.into_pipeline_data())
            }
            PipelineData::Value(v, ..)
                if !matches!(v, Value::List { .. } | Value::Range { .. }) =>
            {
                Ok(v.into_pipeline_data())
            }
            pipe_data => {
                let mut v: Vec<_> = pipe_data.into_iter().collect();
                v.shuffle(&mut thread_rng());
                let iter = v.into_iter();
                Ok(iter
                    .into_pipeline_data(engine_state.ctrlc.clone())
                    .set_metadata(metadata))
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Shuffle rows randomly (execute it several times and see the difference)",
            example: r#"[[version patch]; ['1.0.0' false] ['3.0.1' true] ['2.0.0' false]] | shuffle"#,
            result: None,
        }]
    }
}
