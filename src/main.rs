use nu_plugin::{serve_plugin, EvaluatedCall, LabeledError, MsgPackSerializer, Plugin};
use nu_protocol::{Category, PluginExample, PluginSignature, Span, Spanned, SyntaxShape, Value};

use std::os::unix::process::CommandExt;

struct Implementation;

impl Implementation {
    fn new() -> Self {
        Self {}
    }
}

impl Plugin for Implementation {
    fn signature(&self) -> Vec<PluginSignature> {
        vec![PluginSignature::build("bg")
            .usage("Start a process in the background.")
            .required(
                "command",
                SyntaxShape::String,
                "The command to start in the background.",
            )
            .named(
                "arguments",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "The arguments of the command.",
                Some('a'),
            )
            .switch("debug", "Debug mode", Some('d'))
            .category(Category::Experimental)
            .plugin_examples(vec![PluginExample {
                description: "Start a command in the background".into(),
                example: "some_command --arguments [arg1 --arg2 3]".into(),
                result: None,
            }])]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        assert_eq!(name, "bg");
        let cmd: Option<Spanned<String>> = call.opt(0)?;
        let rest: Option<Vec<String>> = call.get_flag("arguments")?;
        let debug: bool = call.has_flag("debug");

        let ret_val = launch_bg_process(cmd, rest, debug, call.head);

        ret_val
    }
}

fn main() {
    serve_plugin(&mut Implementation::new(), MsgPackSerializer);
}

pub fn launch_bg_process(
    cmd: Option<Spanned<String>>,
    args: Option<Vec<String>>,
    debug: bool,
    value_span: Span,
) -> Result<Value, LabeledError> {
    if let Some(cmd_name) = cmd {
        if let Some(cmd_args) = args {
            if debug {
                eprintln!(
                    "Starting process: '{}' with args '{:?}'",
                    cmd_name.item, cmd_args
                );
            }
            // Start the task as a background child process with arguments
            let _ = std::process::Command::new(&cmd_name.item)
                .args(&cmd_args)
                .process_group(0)
                .spawn()
                .map_err(|err| LabeledError {
                    label: "Could not start process".into(),
                    msg: format!(
                        "Could not start process name: '{}' with args {:?} {}",
                        cmd_name.item, cmd_args, err
                    ),
                    span: Some(cmd_name.span),
                })?;
        } else {
            if debug {
                eprintln!("Starting process: '{}', no arguments", cmd_name.item);
            }
            // Start the task as a background child process without arguments
            let _ = std::process::Command::new(&cmd_name.item)
                .process_group(0)
                .spawn()
                .map_err(|err| LabeledError {
                    label: "Could not start process".into(),
                    msg: format!("Could not start process name: '{}', {}", cmd_name.item, err),
                    span: Some(cmd_name.span),
                })?;
        }
    } else {
        return Err(LabeledError {
            label: "No command provided".into(),
            msg: "No command provided".into(),
            span: Some(value_span),
        });
    }

    Ok(Value::test_nothing())
}
