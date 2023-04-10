mod animations;
mod coaster;
mod telnet;


use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use futures::StreamExt;
use futures::stream::FuturesUnordered;
use serde::{Deserialize, Serialize};
use tokio::io::{BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use toml;

use crate::telnet::{ask_can_do_terminal_type, process_command, receive_u8};


#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct Config {
    pub sockets: Vec<SocketConfig>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct SocketConfig {
    pub listen_socket_addr: SocketAddr,
    pub animation: String,
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


async fn handle_connection(socket: TcpStream, addr: SocketAddr, config: SocketConfig) -> Result<(), telnet::Error> {
    let (reader, writer) = socket.into_split();
    let mut reader_buf = BufReader::new(reader);
    let writer_buf = BufWriter::new(writer);
    let writer_buf_mutex = Arc::new(Mutex::new(writer_buf));

    {
        let mut writer_guard = writer_buf_mutex.lock().await;
        // "can you do terminal type?"
        ask_can_do_terminal_type(&mut *writer_guard, addr).await?;
    }

    loop {
        let rd = receive_u8(&mut reader_buf, addr).await?;
        if rd == telnet::IAC {
            process_command(&mut reader_buf, Arc::clone(&writer_buf_mutex), addr, config.clone()).await?;
        }
    }
}


async fn accept_connection(listener: &TcpListener, socket_config: SocketConfig) -> (TcpStream, SocketAddr, SocketConfig) {
    let (stream, addr) = listener.accept().await
        .expect("failed to accept connection");
    (stream, addr, socket_config)
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

    let mut listeners_configs = Vec::with_capacity(config.sockets.len());
    for socket_config in &config.sockets {
        let listener = TcpListener::bind(socket_config.listen_socket_addr).await
            .expect("failed to bind listener");
        listeners_configs.push((listener, socket_config.clone()));
    }

    loop {
        let mut awaiters = FuturesUnordered::new();
        for (listener, config) in &listeners_configs {
            awaiters.push(accept_connection(listener, config.clone()));
        }

        let (socket, addr, config) = awaiters.next().await.unwrap();
        tokio::spawn(async move {
            handle_connection(socket, addr, config).await
        });
    }
}


#[tokio::main]
async fn main() {
    std::process::exit(run().await);
}
