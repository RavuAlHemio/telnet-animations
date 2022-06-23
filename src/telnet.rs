use std::net::SocketAddr;
use std::ops::DerefMut;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;


/// Interpret As Command (escape sequence)
pub const IAC: u8 = 255;

/// Subnegotiation End
pub const SE: u8 = 240;

/// Subnegotiation Begin
pub const SB: u8 = 250;
pub const WILL: u8 = 251;
pub const WONT: u8 = 252;
pub const DO: u8 = 253;
pub const DONT: u8 = 254;

pub mod option {
    pub const TERMINAL_TYPE: u8 = 24;
    pub const NEGO_WIN_SIZE: u8 = 31;
}

pub mod termtype {
    pub const IS: u8 = 0;
    pub const SEND: u8 = 1;
}


pub(crate) async fn read<R: AsyncRead + Unpin>(mut source: R, addr: SocketAddr, buf: &mut [u8]) -> Option<usize> {
    match source.read(buf).await {
        Ok(r) => Some(r),
        Err(e) => {
            eprintln!("{}: error reading from socket: {}", addr, e);
            None
        },
    }
}

pub(crate) async fn read_u8<R: AsyncRead + Unpin>(mut source: R, addr: SocketAddr) -> Option<u8> {
    match source.read_u8().await {
        Ok(b) => Some(b),
        Err(e) => {
            eprintln!("{}: error reading from socket: {}", addr, e);
            None
        },
    }
}

pub(crate) async fn read_exact<R: AsyncRead + Unpin>(mut source: R, addr: SocketAddr, buf: &mut [u8]) -> Option<()> {
    if let Err(e) = source.read_exact(buf).await {
        eprintln!("{}: error reading from socket: {}", addr, e);
        None
    } else {
        Some(())
    }
}


pub(crate) async fn write_all<W: AsyncWrite + Unpin>(mut target: W, addr: SocketAddr, buf: &[u8]) -> Option<()> {
    if let Err(e) = target.write_all(buf).await {
        eprintln!("{}: error writing to socket: {}", addr, e);
        None
    } else {
        Some(())
    }
}


pub(crate) async fn process_command<'r>(mut reader: &'r mut OwnedReadHalf, writer: Arc<Mutex<OwnedWriteHalf>>, addr: SocketAddr) -> Option<()> {
    let cmd_byte = read_u8(&mut reader, addr).await?;
    if [DO, DONT, WILL, WONT].contains(&cmd_byte) {
        // obtain feature ID
        let option_byte = read_u8(&mut reader, addr).await?;

        match cmd_byte {
            DO => {
                // client wants us to use a feature
                match option_byte {
                    _ => {
                        eprintln!("unexpected DO option {} (0x{:02x})", option_byte, option_byte);
                    },
                }
            },
            DONT => {
                // client does not want us to use a feature
                match option_byte {
                    _ => {
                        eprintln!("unexpected WILL option {} (0x{:02x})", option_byte, option_byte);
                    },
                }
            },
            WILL => {
                // client is ready to use a feature
                match option_byte {
                    option::NEGO_WIN_SIZE => {
                        // sure, go ahead
                        let mut writer_guard = writer.lock().await;
                        write_all(writer_guard.deref_mut(), addr, &[IAC, DO, option_byte]).await?;
                    },
                    option::TERMINAL_TYPE => {
                        // okay, query the terminal type
                        let mut writer_guard = writer.lock().await;
                        write_all(writer_guard.deref_mut(), addr, &[IAC, SB, option::TERMINAL_TYPE, termtype::SEND, IAC, SE]).await?;
                    },
                    _ => {
                        eprintln!("unexpected WILL option {} (0x{:02x})", option_byte, option_byte);
                    },
                }
            },
            WONT => {
                // client is not ready to use a feature
                match option_byte {
                    option::TERMINAL_TYPE => {
                        // fine, assume ANSI
                        // start the animation
                        let writer_copy = Arc::clone(&writer);
                        tokio::spawn(async move {
                            crate::roflcopter::run(writer_copy, addr).await;
                        });
                    },
                    _ => {
                        eprintln!("unexpected WONT option {} (0x{:02x})", option_byte, option_byte);
                    },
                }
            },
            _ => unreachable!(),
        }
    } else if cmd_byte == SB {
        // client is sending additional negotiation information

        // keep reading until we get IAC
        let mut buf = Vec::new();
        loop {
            let b = read_u8(&mut reader, addr).await?;
            if b == IAC {
                // read another byte
                let cmd = read_u8(&mut reader, addr).await?;
                match cmd {
                    SE => break, // alright, it's over
                    IAC => buf.push(b), // escaped IAC
                    other => {
                        eprintln!("unexpected 0x{:02x} following IAC within subnego", other);
                        return None;
                    },
                }
            } else {
                buf.push(b);
            }
        }

        // okay, what do we have?
        if buf.len() == 0 {
            eprintln!("no subnego command?!");
            return None;
        }
        let option_byte = buf[0];
        match option_byte {
            option::TERMINAL_TYPE => {
                if buf.len() == 1 {
                    eprintln!("no termtype subnego subcomand?!");
                    return None;
                }

                let subcommand_byte = buf[1];
                if subcommand_byte != termtype::IS {
                    eprintln!("termtype subnego subcommand is 0x{:02x}, expected 0x{:02x}", subcommand_byte, termtype::IS);
                    return None;
                }

                // the rest is the terminal type
                let term_type = &buf[2..];

                let term_type_string: String = term_type
                    .iter()
                    .map(|c| (*c) as char)
                    .collect();
                eprintln!("term type is {:?}", term_type_string);

                // start the animation
                let writer_copy = Arc::clone(&writer);
                tokio::spawn(async move {
                    crate::roflcopter::run(writer_copy, addr).await;
                });
            },
            option::NEGO_WIN_SIZE => {
                // should be five bytes (including option)
                if buf.len() != 5 {
                    eprintln!("subnego NEGO_WIN_SIZE but buf has {} instead of 5 bytes", buf.len());
                    return None;
                }
                let cols = u16::from_be_bytes(buf[1..3].try_into().unwrap());
                let rows = u16::from_be_bytes(buf[3..5].try_into().unwrap());
                eprintln!("client terminal has {} columns and {} rows", cols, rows);
            },
            other => {
                eprintln!("unexpected subnego command {} (0x{:02x})", other, other);
            },
        }
    }
    Some(())
}
