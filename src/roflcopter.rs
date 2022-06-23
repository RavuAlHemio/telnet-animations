use std::net::SocketAddr;
use std::ops::DerefMut;
use std::sync::Arc;

use tokio::io::AsyncWrite;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::Mutex;

use crate::telnet::write_all;


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


async fn base_frame<W: AsyncWrite + Unpin>(mut target: W, addr: SocketAddr) -> Option<()> {
    // clear screen
    write_all(&mut target, addr, b"\x1B[2J").await?;

    // go to top left
    write_all(&mut target, addr, b"\x1B[H").await?;

    // output roflcopter
    write_all(&mut target, addr, ROFLCOPTER_BASE.as_bytes()).await?;

    Some(())
}

async fn frame0<W: AsyncWrite + Unpin>(mut target: W, addr: SocketAddr) -> Option<()> {
    // remove upper rotors
    // => top left
    write_all(&mut target, addr, b"\x1B[H").await?;
    // => space over
    write_all(&mut target, addr, b"     ").await?;
    // => top right
    write_all(&mut target, addr, b"\x1B[1;19H").await?;
    // => space over
    write_all(&mut target, addr, b"     ").await?;

    // vertical blades
    write_all(&mut target, addr, b"\x1B[3;2H").await?;
    write_all(&mut target, addr, b" L ").await?;
    write_all(&mut target, addr, b"\x1B[4;2H").await?;
    write_all(&mut target, addr, b" O ").await?;
    write_all(&mut target, addr, b"\x1B[5;2H").await?;
    write_all(&mut target, addr, b" L ").await?;

    Some(())
}

async fn frame1<W: AsyncWrite + Unpin>(mut target: W, addr: SocketAddr) -> Option<()> {
    // add upper rotors
    // => top left
    write_all(&mut target, addr, b"\x1B[H").await?;
    // => space over
    write_all(&mut target, addr, b"ROFL:").await?;
    // => top right
    write_all(&mut target, addr, b"\x1B[1;19H").await?;
    // => space over
    write_all(&mut target, addr, b":ROFL").await?;

    // horizontal blades
    write_all(&mut target, addr, b"\x1B[3;2H").await?;
    write_all(&mut target, addr, b"   ").await?;
    write_all(&mut target, addr, b"\x1B[4;2H").await?;
    write_all(&mut target, addr, b"LOL").await?;
    write_all(&mut target, addr, b"\x1B[5;2H").await?;
    write_all(&mut target, addr, b"   ").await?;

    Some(())
}

pub(crate) async fn run(writer: Arc<Mutex<OwnedWriteHalf>>, addr: SocketAddr) -> Option<()> {
    {
        let mut writer_guard = writer.lock().await;
        base_frame(writer_guard.deref_mut(), addr).await?;
    }

    loop {
        {
            let mut writer_guard = writer.lock().await;
            frame0(writer_guard.deref_mut(), addr).await?;
        }

        {
            let mut writer_guard = writer.lock().await;
            frame1(writer_guard.deref_mut(), addr).await?;
        }
    }
}
