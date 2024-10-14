use tokio_stream::{StreamExt, wrappers::LinesStream};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::Receiver;

use command_group::{AsyncCommandGroup, UnixChildExt, Signal};
use tokio::process::Command;
use std::process::Stdio;

use crate::log::{Logger, LogRecord, LogStream, ProcessLogRecord, ControllerLogRecord};
use crate::cfg::Process;

pub async fn run(
  name: &str,
  proc_cfg:  Process,
  mut signal_rx: Receiver<Signal>,
  logger: &Logger,
) -> anyhow::Result<bool> {
  let cwd = std::env::current_dir()?;
  let directory = proc_cfg.directory.unwrap_or(cwd);

  let mut child = match proc_cfg.shell {
    Some(sh) => {
      Command::new(sh).arg("-c").arg(proc_cfg.command)
        .current_dir(directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .group_spawn()?
    },
    None => {
      let parts = shell_words::split(&proc_cfg.command)?;
      let command = parts.first().ok_or(anyhow!("empty command"))?;
      let args = parts.iter().skip(1).collect::<Vec<_>>();

      Command::new(command)
        .args(args)
        .current_dir(directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .group_spawn()?
    },
  };

  let stdout = child.inner().stdout.take().ok_or(anyhow!("could not pipe stdout"))?;
  let stderr = child.inner().stderr.take().ok_or(anyhow!("could not pipe stderr"))?;

  let stdout_reader = BufReader::new(stdout);
  let stderr_reader = BufReader::new(stderr);

  let mut stdout_lines = LinesStream::new(stdout_reader.lines());
  let mut stderr_lines = LinesStream::new(stderr_reader.lines());

  {
    let name = name.to_string();
    let logger = logger.clone();

    tokio::spawn(async move {
      while let Some(line) = stdout_lines.next().await {
        if let Ok(line) = line {
          logger.log(LogRecord::Process {
            stream: LogStream::Stdout,
            record: ProcessLogRecord {
              name: name.clone(),
              line,
            },
          });
        }
      }
    });
  }

  {
    let name = name.to_string();
    let logger = logger.clone();

    tokio::spawn(async move {
      while let Some(line) = stderr_lines.next().await {
        if let Ok(line) = line {
          logger.log(LogRecord::Process {
            stream: LogStream::Stderr,
            record: ProcessLogRecord {
              name: name.clone(),
              line,
            },
          });
        }
      }
    });
  }

  let success = loop {
    tokio::select!{
      res = child.wait() => {
        let success = match res {
          Ok(status) => {
            logger.log(LogRecord::Controller {
              stream: LogStream::Stdout,
              record: ControllerLogRecord::new(
                format!("{} exited with {}", name, status),
              ),
            });

            match status.code() {
              Some(0) => true,
              Some(_) => false,
              None => true,
            }
          },
          Err(err) => {
            logger.log(LogRecord::Controller {
              stream: LogStream::Stderr,
              record: ControllerLogRecord::new(format!("{}: {}", name, err)),
            });

            false
          },
        };

        break success;
      }
      sig = signal_rx.recv() => {
        let sig = sig.ok_or(anyhow!("signal channel closed"))?;
        child.signal(sig).context("failed to broadcast signal")?;
      }
    }
  };

  Ok(success)
}
