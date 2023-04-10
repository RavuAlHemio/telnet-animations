use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::BufWriter;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::coaster::{decode_movements, Rollercoaster};
use crate::telnet;


const LOLLERCOASTER_BASE: &str = concat!(
    "                      THE ULTIMATE LOLLERCOASTER\n",
    "_____\n",
    "     \\         ___     (sponsored by LMAONADE)\n",
    "      \\       /   \\\n",
    "       \\     /    |\n",
    "        \\___/     |\n",
    "                  |      ___\n",
    "                  A     /   \\\n",
    "                  H    /     \\\n",
    "                  V    |     |     ___\n",
    "                  |    |     /    /   \\\n",
    "        __________|____|____/    /     \\\n",
    "       /          |    \\        /      |\n",
    "       |          /     \\______/       A\n",
    "       \\__<I>____/            ___      V\n",
    "                             /   \\     |\n",
    "                             |    \\    /\n",
    "                             A     \\__/\n",
    "                             V\n",
    "                             |\n",
    "                             \\\n",
    "                              \\___________________",
);
const LOLLERCOASTER_MOVEMENTS: &str = concat!(
    "66666666333366999666632222",
    "11222214444414447889666666",
    "66666666666666988744122223",
    "66666999966663321212147774",
    "44412323236666666666666666",
    "666666",
);
const SLEEP_DURATION: Duration = Duration::from_millis(50);


pub(crate) async fn run(writer: Arc<Mutex<BufWriter<OwnedWriteHalf>>>, addr: SocketAddr) -> Result<(), telnet::Error> {
    let base_lines: Vec<String> = LOLLERCOASTER_BASE
        .split('\n')
        .map(|bl| bl.to_owned())
        .collect();
    let mut coaster = Rollercoaster::new(
        base_lines,
        "LOL".to_owned(),
        vec![(1, -3), (1, -2), (1, -1)],
        decode_movements(LOLLERCOASTER_MOVEMENTS).unwrap(),
    );

    loop {
        coaster.reset();

        {
            let mut writer_guard = writer.lock().await;
            let base_frame = coaster.get_base_frame();

            // clear screen
            telnet::write_all(&mut *writer_guard, addr, b"\x1B[2J").await?;

            // go to top left
            telnet::write_all(&mut *writer_guard, addr, b"\x1B[H").await?;

            // output base frame
            telnet::write_all(&mut *writer_guard, addr, base_frame.as_bytes()).await?;
            telnet::flush(&mut *writer_guard, addr).await?;
        }

        while let Some(new_commands) = coaster.advance() {
            let mut writer_guard = writer.lock().await;
            telnet::write_all(&mut *writer_guard, addr, new_commands.as_bytes()).await?;
            telnet::flush(&mut *writer_guard, addr).await?;

            sleep(SLEEP_DURATION).await;
        }

        // reset and start again
    }
}
