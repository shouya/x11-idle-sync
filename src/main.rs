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
  #[arg(short, long, default_value_t = 5)]
  idle_threshold: u64,
}

#[proxy(
  interface = "org.freedesktop.login1.Session",
  default_service = "org.freedesktop.login1"
)]
trait Login1Session {
  fn set_idle_hint(&self, idle: bool) -> zbus::Result<()>;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args = Args::parse();
  let idle_threshold = Duration::from_secs(args.idle_threshold);

  let (conn, screen_num) = Connection::connect(None)?;
  let setup = conn.get_setup();
  let screen = setup.roots().nth(screen_num as usize).unwrap();
  let root = screen.root();

  // Create a proxy for the session
  let zbus_conn = ZbusConnection::system().await?;
  let session_proxy = Login1SessionProxy::builder(&zbus_conn)
    .path("/org/freedesktop/login1/session/self")?
    .build()
    .await?;

  let check_interval = (idle_threshold / 10).max(Duration::from_secs(5));

  println!(
    "x11-idle-sync started with idle threshold of {} seconds",
    args.idle_threshold
  );

  let mut state = false;
  loop {
    let idle = get_idle_duration(&conn, root)?;
    let new_state = idle >= idle_threshold;
    session_proxy.set_idle_hint(new_state).await?;

    if new_state != state {
      println!("User is {}", if new_state { "idle" } else { "active" });
    }

    state = new_state;
    sleep(check_interval).await;
  }
}

fn get_idle_duration(
  conn: &Connection,
  root: Window,
) -> Result<Duration, Box<dyn std::error::Error>> {
  let cookie = conn.send_request(&screensaver::QueryInfo {
    drawable: Drawable::Window(root),
  });
  let reply = conn.wait_for_reply(cookie)?;
  let idle_ms = reply.ms_since_user_input();
  let duration = Duration::from_millis(idle_ms as u64);
  Ok(duration)
}
