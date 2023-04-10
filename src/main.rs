mod roflcopter;
mod telnet;


use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use toml;

use crate::telnet::{process_command, read_u8, write_all};


#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct Config {
    pub listen_socket_addr: SocketAddr,
}


fn output_usage() {
    eprintln!("Usage: telnet-animations [CONFIG.TOML]");
}


fn hexdump(prefix: &str, buf: &[u8]) {
    for i in (0..buf.len()).step_by(16) {
        print!("{}: {:08x}", prefix, i);
        for j in 0..16 {
            if i + j < buf.len() {
                print!(" {:02x}", buf[i + j]);
            } else {
                print!("   ");
            }
        }
        for j in 0..16 {
            if i + j < buf.len() {
                if buf[i+j] >= 0x20 && buf[i+j] <= 0x7E {
                    print!("{}", buf[i+j] as char);
                } else {
                    print!(".");
                }
            }
        }
        println!();
    }
}


async fn handle_connection(mut socket: TcpStream, addr: SocketAddr) -> Option<()> {
    // "can you do terminal type?"
    let can_term_type_ask_buf = [telnet::IAC, telnet::DO, telnet::option::TERMINAL_TYPE];
    write_all(&mut socket, addr, &can_term_type_ask_buf).await?;

    let (mut reader, writer) = socket.into_split();
    let writer_lock = Arc::new(Mutex::new(writer));

    loop {
        let rd = read_u8(&mut reader, addr).await?;
        if rd == telnet::IAC {
            process_command(&mut reader, Arc::clone(&writer_lock), addr).await?;
        }
    }
}


async fn run() -> i32 {
    let args: Vec<OsString> = env::args_os().collect();
    if args.len() > 1 && args[1] == "--help" {
        output_usage();
        return 1;
    }
    if args.len() > 2 {
        output_usage();
        return 1;
    }

    let config_file_name = if let Some(fn_arg) = args.get(1) {
        PathBuf::from(fn_arg)
    } else {
        PathBuf::from("config.toml")
    };

    // load config
    let config: Config = {
        let mut f = File::open(&config_file_name)
            .expect("failed to open config file");
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)
            .expect("failed to read config file");
        let string = String::from_utf8(buf)
            .expect("failed to decode config file as UTF-8");
        toml::from_str(&string)
            .expect("failed to parse config file")
    };

    let listener = TcpListener::bind(config.listen_socket_addr).await
        .expect("failed to bind listener");
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                tokio::spawn(async move {
                    handle_connection(socket, addr).await
                });
            },
            Err(e) => {
                eprintln!("failed to accept connection: {}", e);
            }
        }
    }
}


#[tokio::main]
async fn main() {
    std::process::exit(run().await);
}
