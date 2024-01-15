use nu_plugin::{serve_plugin, EvaluatedCall, LabeledError, MsgPackSerializer, Plugin};
use nu_protocol::{Category, PluginExample, PluginSignature, Span, Spanned, SyntaxShape, Value};
#[cfg(unix)]
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
            .switch("pid", "Return process ID", Some('p'))
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
        _config: &Option<Value>,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        assert_eq!(name, "bg");
        let cmd: Spanned<String> = call.req(0)?;
        let rest: Option<Vec<String>> = call.get_flag("arguments")?;
        let debug: bool = call.has_flag("debug");
        let pid: bool = call.has_flag("pid");

        launch_bg_process(cmd, rest, debug, pid, call.head)
    }
}

fn main() {
    serve_plugin(&mut Implementation::new(), MsgPackSerializer);
}

pub fn launch_bg_process(
    cmd_name: Spanned<String>,
    args: Option<Vec<String>>,
    debug: bool,
    pid: bool,
    value_span: Span,
) -> Result<Value, LabeledError> {
    let debug_name = if let Some(ref cmd_args) = args {
        format!("'{}' with args {:?}", cmd_name.item, cmd_args)
    } else {
        format!("'{}'", cmd_name.item)
    };

    if debug {
        eprintln!("Starting process {debug_name}");
    }

    let mut p = std::process::Command::new(&cmd_name.item);
    if let Some(cmd_args) = args {
        p.args(cmd_args);
    }
    #[cfg(unix)]
    p.process_group(0);

    let process = p.spawn().map_err(|err| LabeledError {
        label: "Could not start process".into(),
        msg: format!("Could not start process {debug_name}: {err}",),
        span: Some(cmd_name.span),
    })?;

    if pid {
        Ok(Value::Int {
            val: process.id() as i64,
            internal_span: value_span,
        })
    } else {
        Ok(Value::nothing(value_span))
    }
}
