use tokio::sync::mpsc::Sender;

use tokio::signal::unix::{signal, SignalKind};
use command_group::Signal;

use crate::log::{LogRecord, Logger, LogStream, ControllerLogRecord};


pub async fn listen(
  signal_tx: Sender<Signal>,
  logger: &Logger,
) -> anyhow::Result<()> {
  let mut sigint = signal(SignalKind::interrupt())?;
  let mut sigterm = signal(SignalKind::terminate())?;
  let mut sighup = signal(SignalKind::hangup())?;

  loop {
    tokio::select! {
      _ = sigint.recv() => {
        logger.log(
          LogRecord::Controller {
            stream: LogStream::Stdout,
            record: ControllerLogRecord::new("Received SIGINT".to_string()),
          }
        );
        signal_tx.send(Signal::SIGINT).await?;
        break
      }
      _ = sigterm.recv() => {
        logger.log(
          LogRecord::Controller {
            stream: LogStream::Stdout,
            record: ControllerLogRecord::new("Received SIGTERM".to_string()),
          }
        );
        signal_tx.send(Signal::SIGTERM).await?;
        break
      }
      _ = sighup.recv() => {
        logger.log(
          LogRecord::Controller {
            stream: LogStream::Stdout,
            record: ControllerLogRecord::new("Received SIGHUP".to_string()),
          }
        );
        signal_tx.send(Signal::SIGHUP).await?;
      }
    }
  }

  Ok(())
}
