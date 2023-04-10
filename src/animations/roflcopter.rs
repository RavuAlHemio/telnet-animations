use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::BufWriter;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::Mutex;

use crate::telnet;


const ROFLCOPTER_BASE: &str = concat!(
    "ROFL:ROFL:LOL:ROFL:ROFL\r\n",
    "           ^\r\n",
    "  L  /-----------\r\n",
    " LOL===       [] \\\r\n",
    "  L    \\          \\\r\n",
    "        \\__________]\r\n",
    "            I   I\r\n",
    "         -----------/\r\n",
);


async fn base_frame(writer: &mut BufWriter<OwnedWriteHalf>, addr: SocketAddr) -> Result<(), telnet::Error> {
    // clear screen
    telnet::write_all(writer, addr, b"\x1B[2J").await?;

    // go to top left
    telnet::write_all(writer, addr, b"\x1B[H").await?;

    // output roflcopter
    telnet::write_all(writer, addr, ROFLCOPTER_BASE.as_bytes()).await?;

    telnet::flush(writer, addr).await
}

async fn frame0(writer: &mut BufWriter<OwnedWriteHalf>, addr: SocketAddr) -> Result<(), telnet::Error> {
    // remove upper rotors
    // => top left
    telnet::write_all(writer, addr, b"\x1B[H").await?;
    // => space over
    telnet::write_all(writer, addr, b"     ").await?;
    // => top right
    telnet::write_all(writer, addr, b"\x1B[1;19H").await?;
    // => space over
    telnet::write_all(writer, addr, b"     ").await?;

    // vertical blades
    telnet::write_all(writer, addr, b"\x1B[3;2H").await?;
    telnet::write_all(writer, addr, b" L ").await?;
    telnet::write_all(writer, addr, b"\x1B[4;2H").await?;
    telnet::write_all(writer, addr, b" O ").await?;
    telnet::write_all(writer, addr, b"\x1B[5;2H").await?;
    telnet::write_all(writer, addr, b" L ").await?;

    telnet::flush(writer, addr).await
}

async fn frame1(writer: &mut BufWriter<OwnedWriteHalf>, addr: SocketAddr) -> Result<(), telnet::Error> {
    // add upper rotors
    // => top left
    telnet::write_all(writer, addr, b"\x1B[H").await?;
    // => space over
    telnet::write_all(writer, addr, b"ROFL:").await?;
    // => top right
    telnet::write_all(writer, addr, b"\x1B[1;19H").await?;
    // => space over
    telnet::write_all(writer, addr, b":ROFL").await?;

    // horizontal blades
    telnet::write_all(writer, addr, b"\x1B[3;2H").await?;
    telnet::write_all(writer, addr, b"   ").await?;
    telnet::write_all(writer, addr, b"\x1B[4;2H").await?;
    telnet::write_all(writer, addr, b"LOL").await?;
    telnet::write_all(writer, addr, b"\x1B[5;2H").await?;
    telnet::write_all(writer, addr, b"   ").await?;

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

        {
            let mut writer_guard = writer.lock().await;
            frame1(&mut *writer_guard, addr).await?;
        }
    }
}
