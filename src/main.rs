use tokio::{sync::mpsc::channel, task::JoinSet};
use command_group::Signal;

use std::path::PathBuf;
use clap::Parser;


#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
  config_path: PathBuf,
}

mod cfg;
mod log;
mod proc;
mod sig;

#[tokio::main]
async fn main() {
  let args = Cli::parse();
  let logger = log::Logger::new();

  match run(args, logger).await {
    Ok(success) => {
      if success {
        std::process::exit(0);
      } else {
        std::process::exit(1);
      }
    },
    Err(err) => {
      eprintln!("{}", err);
      std::process::exit(1);
    },
  }
}

async fn run(args: Cli, mut logger: log::Logger) -> anyhow::Result<bool> {
  let config = cfg::Config::load(&args.config_path)?;

  for (name, _) in config.processes.iter() {
    logger.register_name(name);
  }

  let mut tasks = JoinSet::new();

  let (exit_tx, mut exit_rx) = channel(config.processes.len());
  let mut signal_senders = vec![];

  for (name, process) in config.processes.iter() {
    let (signal_tx, signal_rx) = channel(1);
    let exit_tx = exit_tx.clone();

    let logger = logger.clone();
    let name = name.clone();
    let proc_cfg = process.clone();

    tasks.spawn(async move {
      let res = proc::run(&name, proc_cfg, signal_rx, &logger).await;

      let success = match res {
        Ok(success) => success,
        Err(err) =>{
          logger.log(log::LogRecord::Controller {
            stream: log::LogStream::Stderr,
            record: log::ControllerLogRecord::new(err.to_string()),
          });

          false
        },
      };

      match exit_tx.send(success).await {
        Ok(_) => {},
        Err(_) => {
          logger.log(log::LogRecord::Controller {
            stream: log::LogStream::Stderr,
            record: log::ControllerLogRecord::new("exit channel closed".to_string()),
          });

          std::process::exit(2);
        },
      }
    });

    signal_senders.push(signal_tx);
  }

  {
    let logger = logger.clone();
    let signal_senders = signal_senders.clone();
    let (signal_tx, mut signal_rx) = channel(1);

    tokio::spawn(async move {
      match sig::listen(signal_tx, &logger).await {
        Ok(_) => {},
        Err(err) => {
          logger.log(log::LogRecord::Controller {
            stream: log::LogStream::Stderr,
            record: log::ControllerLogRecord::new(err.to_string()),
          });

          std::process::exit(2);
        },
      }
    });

    tokio::spawn(async move {
      while let Some(sig) = signal_rx.recv().await {
        for sender in signal_senders.iter() {
          match sender.send(sig).await {
            Ok(_) => {},
            Err(_) => {
              logger.log(log::LogRecord::Controller {
                stream: log::LogStream::Stderr,
                record: log::ControllerLogRecord::new("signal channel closed".to_string()),
              });
            },
          }
        }
      }
    });
  }

  let success = match exit_rx.recv().await {
    Some(success) => success,
    None => {
      logger.log(log::LogRecord::Controller {
        stream: log::LogStream::Stderr,
        record: log::ControllerLogRecord::new("exit channel closed".to_string()),
      });

      false
    },
  };

  for sender in signal_senders.iter() {
    if let Err(_) = sender.send(Signal::SIGTERM).await {
      // do nothing, the process was already terminated
    }
  }

  while let Some(res) = tasks.join_next().await {
    if let Err(err) = res {
      logger.log(log::LogRecord::Controller {
        stream: log::LogStream::Stderr,
        record: log::ControllerLogRecord::new(err.to_string()),
      });
    }
  }

  Ok(success)
}
