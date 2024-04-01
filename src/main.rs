use nu_plugin::{
    serve_plugin, EngineInterface, EvaluatedCall, MsgPackSerializer, Plugin, PluginCommand,
    SimplePluginCommand,
};
use nu_protocol::{Category, Example, LabeledError, Signature, Span, Spanned, SyntaxShape, Value};
#[cfg(unix)]
use std::os::unix::process::CommandExt;

struct BgPlugin;

impl Plugin for BgPlugin {
    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(Implementation)]
    }
}

struct Implementation;

impl SimplePluginCommand for Implementation {
    type Plugin = BgPlugin;

    fn name(&self) -> &str {
        "bg"
    }

    fn usage(&self) -> &str {
        "Start a process in the background."
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
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
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Start a command in the background".into(),
            example: "some_command --arguments [arg1 --arg2 3]".into(),
            result: None,
        }]
    }

    fn run(
        &self,
        _plugin: &BgPlugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        let cmd: Spanned<String> = call.req(0)?;
        let rest: Option<Vec<String>> = call.get_flag("arguments")?;
        let debug: bool = call.has_flag("debug")?;
        let pid: bool = call.has_flag("pid")?;

        launch_bg_process(cmd, rest, debug, pid, call.head)
    }
}

fn main() {
    serve_plugin(&BgPlugin, MsgPackSerializer);
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

    let process = p.spawn().map_err(|err| {
        LabeledError::new(format!("Could not start process {debug_name}: {err}"))
            .with_label("Could not start process", cmd_name.span)
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
