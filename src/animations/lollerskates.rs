use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::BufWriter;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::telnet;


const LOLLERSKATES_BASE: &str = concat!(
    "        /\\O\r\n",
    "         /\\/\r\n",
    "        /\\\r\n",
    "       /  \\\r\n",
    "      LOL LOL\r\n",
    ":-D LOLLERSKATES :-D\r\n",
);
const SLEEP_DURATION: Duration = Duration::from_millis(100);


async fn base_frame(writer: &mut BufWriter<OwnedWriteHalf>, addr: SocketAddr) -> Result<(), telnet::Error> {
    // clear screen
    telnet::write_all(writer, addr, b"\x1B[2J").await?;

    // go to top left
    telnet::write_all(writer, addr, b"\x1B[H").await?;

    // output lollerskater
    telnet::write_all(writer, addr, LOLLERSKATES_BASE.as_bytes()).await?;

    telnet::flush(writer, addr).await
}

async fn frame0(writer: &mut BufWriter<OwnedWriteHalf>, addr: SocketAddr) -> Result<(), telnet::Error> {
    telnet::write_all(writer, addr, b"\x1B[1;9H").await?;
    telnet::write_all(writer, addr, b" _").await?;

    telnet::write_all(writer, addr, b"\x1B[2;9H").await?;
    telnet::write_all(writer, addr, b"//|_").await?;

    telnet::write_all(writer, addr, b"\x1B[3;9H").await?;
    telnet::write_all(writer, addr, b" |").await?;

    telnet::write_all(writer, addr, b"\x1B[4;8H").await?;
    telnet::write_all(writer, addr, b" /| ").await?;

    telnet::write_all(writer, addr, b"\x1B[5;7H").await?;
    telnet::write_all(writer, addr, b" LLOL   ").await?;

    telnet::flush(writer, addr).await
}

async fn frame1(writer: &mut BufWriter<OwnedWriteHalf>, addr: SocketAddr) -> Result<(), telnet::Error> {
    telnet::write_all(writer, addr, b"\x1B[1;10H").await?;
    telnet::write_all(writer, addr, b" ").await?;

    telnet::write_all(writer, addr, b"\x1B[2;9H").await?;
    telnet::write_all(writer, addr, b" /_ ").await?;

    telnet::write_all(writer, addr, b"\x1B[3;11H").await?;
    telnet::write_all(writer, addr, b"\\").await?;

    telnet::write_all(writer, addr, b"\x1B[4;10H").await?;
    telnet::write_all(writer, addr, b" |").await?;

    telnet::write_all(writer, addr, b"\x1B[5;9H").await?;
    telnet::write_all(writer, addr, b"OLLOL").await?;

    telnet::flush(writer, addr).await
}

async fn frame2(writer: &mut BufWriter<OwnedWriteHalf>, addr: SocketAddr) -> Result<(), telnet::Error> {
    telnet::write_all(writer, addr, b"\x1B[1;9H").await?;
    telnet::write_all(writer, addr, b"/\\").await?;

    telnet::write_all(writer, addr, b"\x1B[2;11H").await?;
    telnet::write_all(writer, addr, b"\\/").await?;

    telnet::write_all(writer, addr, b"\x1B[3;9H").await?;
    telnet::write_all(writer, addr, b"/\\ ").await?;

    telnet::write_all(writer, addr, b"\x1B[4;8H").await?;
    telnet::write_all(writer, addr, b"/  \\").await?;

    telnet::write_all(writer, addr, b"\x1B[5;7H").await?;
    telnet::write_all(writer, addr, b"LOL LOL").await?;

    telnet::flush(writer, addr).await
}

pub(crate) async fn run(writer: Arc<Mutex<BufWriter<OwnedWriteHalf>>>, addr: SocketAddr) -> Result<(), telnet::Error> {
    {
        let mut writer_guard = writer.lock().await;
        base_frame(&mut *writer_guard, addr).await?;
    }

    loop {
        {
            let mut writer_guard = writer.lock().await;
            frame0(&mut *writer_guard, addr).await?;
        }
        sleep(SLEEP_DURATION).await;

        {
            let mut writer_guard = writer.lock().await;
            frame1(&mut *writer_guard, addr).await?;
        }
        sleep(SLEEP_DURATION).await;

        {
            let mut writer_guard = writer.lock().await;
            frame2(&mut *writer_guard, addr).await?;
        }
        sleep(SLEEP_DURATION).await;
    }
}
