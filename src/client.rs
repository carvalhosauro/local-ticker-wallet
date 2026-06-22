use crate::ipc::{self, Request, Response};
use tokio::io::BufReader;
use tokio::net::UnixStream;
use tokio::time::{sleep, Duration};

pub async fn send(req: Request) -> anyhow::Result<Response> {
    let sock = crate::paths::socket_path();
    let stream = match UnixStream::connect(&sock).await {
        Ok(s) => s,
        Err(_) => {
            spawn_daemon().await?;
            connect_retry(&sock).await?
        }
    };
    let (r, mut w) = stream.into_split();
    let mut reader = BufReader::new(r);
    ipc::write_msg(&mut w, &req).await?;
    let line = ipc::read_line(&mut reader)
        .await?
        .ok_or_else(|| anyhow::anyhow!("daemon closed connection"))?;
    Ok(serde_json::from_str(&line)?)
}

async fn spawn_daemon() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    std::process::Command::new(exe).arg("daemon").spawn()?;
    Ok(())
}

async fn connect_retry(sock: &std::path::Path) -> anyhow::Result<UnixStream> {
    for _ in 0..30 {
        if let Ok(s) = UnixStream::connect(sock).await {
            return Ok(s);
        }
        sleep(Duration::from_millis(100)).await;
    }
    anyhow::bail!("daemon did not start (socket {:?} never appeared)", sock)
}
