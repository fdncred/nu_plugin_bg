// These changes are so close. Unfortunately, the process is not being launched in the background.
// 2024-01-10

use nu_plugin::{serve_plugin, EvaluatedCall, LabeledError, MsgPackSerializer, Plugin};
use nu_protocol::{Category, PluginExample, PluginSignature, Span, Spanned, SyntaxShape, Value};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::{io::Read, process::Stdio};

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

    // capture stdin, stdout, stderr
    // p.stdin(Stdio::piped());
    p.stdout(Stdio::piped());
    p.stderr(Stdio::piped());

    let mut process = p.spawn().map_err(|err| LabeledError {
        label: "Could not start process".into(),
        msg: format!("Could not start process {debug_name}: {err}",),
        span: Some(cmd_name.span),
    })?;

    // let child_stdin = process.stdin.take().unwrap();
    let mut child_stdout = match process.stdout.take() {
        Some(stdout) => stdout,
        None => {
            return Err(LabeledError {
                label: "Could not capture stdout of process".into(),
                msg: format!("Could not capture stdout of process {debug_name}",),
                span: Some(cmd_name.span),
            })
        }
    };
    let mut child_stderr = match process.stderr.take() {
        Some(stderr) => stderr,
        None => {
            return Err(LabeledError {
                label: "Could not capture stderr of process".into(),
                msg: format!("Could not capture stderr of process {debug_name}",),
                span: Some(cmd_name.span),
            })
        }
    };

    let mut stdout_data = vec![];
    child_stdout
        .read_to_end(&mut stdout_data)
        .map_err(|err| LabeledError {
            label: "Could not read stdout of process".into(),
            msg: format!("Could not read stdout of process {debug_name}. Error: {err}"),
            span: Some(cmd_name.span),
        })?;

    let mut stderr_data = vec![];
    child_stderr
        .read_to_end(&mut stderr_data)
        .map_err(|err| LabeledError {
            label: "Could not read stderr of process".into(),
            msg: format!("Could not read stderr of process {debug_name}. Error: {err}"),
            span: Some(cmd_name.span),
        })?;

    let status = process.wait().map_err(|err| LabeledError {
        label: "Could not wait for process".into(),
        msg: format!("Could not wait for process {debug_name}. Error: {err}"),
        span: Some(cmd_name.span),
    })?;

    if status.success() {
        if pid {
            Ok(Value::Int {
                val: process.id() as i64,
                internal_span: value_span,
            })
        } else if stdout_data.is_empty() {
            Ok(Value::nothing(value_span))
        } else {
            Ok(Value::string(
                String::from_utf8_lossy(&stdout_data).to_string(),
                value_span,
            ))
        }
    } else if stderr_data.is_empty() {
        Err(LabeledError {
            label: "Process did not exit successfully".into(),
            msg: format!("Process {debug_name} did not exit successfully. Exit code: {status}",),
            span: Some(cmd_name.span),
        })
    } else {
        Err(LabeledError {
            label: "Process did not exit successfully".into(),
            msg: format!(
                "Process {debug_name} did not exit successfully. Exit code: {status}. Error: {}",
                String::from_utf8_lossy(&stderr_data)
            ),
            span: Some(cmd_name.span),
        })
    }
}
