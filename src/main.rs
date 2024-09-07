use clap::Parser;
use std::time::Duration;
use tokio::time::sleep;
use xcb::{
  screensaver,
  x::{Drawable, Window},
  Connection,
};
use zbus::{proxy, Connection as ZbusConnection};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
  /// Idle threshold in seconds
  #[arg(short = 't', long, default_value_t = 5)]
  idle_threshold: u64,

  /// Disable resetting idle hint to false on exit
  #[arg(short = 'N', long)]
  no_reset_on_exit: bool,

  /// Run as a one-shot idle check (check once and exit)
  #[arg(short = '1', long)]
  one_shot: bool,
}

#[proxy(
  interface = "org.freedesktop.login1.Session",
  default_service = "org.freedesktop.login1"
)]
trait Login1Session {
  fn set_idle_hint(&self, idle: bool) -> zbus::Result<()>;
}

struct IdleMonitor {
  conn: Connection,
  root: Window,
  idle_threshold: Duration,
  check_interval: Duration,
  session_proxy: Login1SessionProxy<'static>,
}

impl IdleMonitor {
  async fn new(
    idle_threshold: Duration,
  ) -> Result<Self, Box<dyn std::error::Error>> {
    let (conn, screen_num) = Connection::connect(None)?;
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();
    let root = screen.root();

    let zbus_conn = ZbusConnection::system().await?;
    let session_proxy = Login1SessionProxy::builder(&zbus_conn)
      .path("/org/freedesktop/login1/session/self")?
      .build()
      .await?;

    let check_interval = (idle_threshold / 10).max(Duration::from_secs(5));

    Ok(Self {
      conn,
      root,
      idle_threshold,
      check_interval,
      session_proxy,
    })
  }

  fn get_idle_duration(&self) -> Result<Duration, Box<dyn std::error::Error>> {
    let cookie = self.conn.send_request(&screensaver::QueryInfo {
      drawable: Drawable::Window(self.root),
    });
    let reply = self.conn.wait_for_reply(cookie)?;
    let idle_ms = reply.ms_since_user_input();
    Ok(Duration::from_millis(idle_ms as u64))
  }

  async fn run(
    &self,
    mut signals: ExitSignals,
  ) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = false;

    loop {
      tokio::select! {
        _ = signals.recv() => {
          println!("Received shutdown signal, exiting idle monitoring loop...");
          break;
        }

        _ = sleep(self.check_interval) => {
          let idle = self.get_idle_duration()?;
          let new_state = idle >= self.idle_threshold;
          self.session_proxy.set_idle_hint(new_state).await?;

          if new_state != state {
            println!("User is {}", if new_state { "idle" } else { "active" });
          }

          state = new_state;
        }
      }
    }

    Ok(())
  }

  async fn one_shot_check(&self) -> Result<(), Box<dyn std::error::Error>> {
    let idle = self.get_idle_duration()?;
    let state = idle >= self.idle_threshold;
    self.session_proxy.set_idle_hint(state).await?;
    println!("User is {}", if state { "idle" } else { "active" });
    Ok(())
  }

  async fn set_idle_hint_false(
    &self,
  ) -> Result<(), Box<dyn std::error::Error>> {
    self.session_proxy.set_idle_hint(false).await?;
    Ok(())
  }
}

pub struct ExitSignals {
  sigint: tokio::signal::unix::Signal,
  sigterm: tokio::signal::unix::Signal,
}

impl ExitSignals {
  pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
    use tokio::signal::unix::{signal, SignalKind};
    let sigterm = signal(SignalKind::terminate())?;
    let sigint = signal(SignalKind::interrupt())?;
    Ok(Self { sigint, sigterm })
  }

  pub async fn recv(&mut self) {
    tokio::select! {
      _ = self.sigterm.recv() => {}
      _ = self.sigint.recv() => {}
    }
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  let exit_signals = ExitSignals::new()?;
  let idle_threshold = Duration::from_secs(args.idle_threshold);

  let idle_monitor = IdleMonitor::new(idle_threshold).await?;

  println!(
    "x11-idle-sync started with idle threshold of {} seconds",
    args.idle_threshold
  );

  if args.one_shot {
    idle_monitor.one_shot_check().await?;
  } else {
    idle_monitor.run(exit_signals).await?;
  }

  // Set idle hint to false before exiting, unless disabled
  if !args.no_reset_on_exit && !args.one_shot {
    idle_monitor.set_idle_hint_false().await?;
    println!("Idle hint set to false. Exiting.");
  }

  Ok(())
}
