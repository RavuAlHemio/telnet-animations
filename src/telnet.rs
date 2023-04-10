//! Implementation of the Telnet protocol.
//!
//! Telnet, as implemented here, is defined mostly in RFC854.


use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;

use crate::Config;


/// Interpret As Command (escape sequence)
pub const IAC: u8 = 255;

/// Subnegotiation End
pub const SE: u8 = 240;

/// Subnegotiation Begin
pub const SB: u8 = 250;

/// Indicates that a party wishes to enable a feature on their end of the session.
pub const WILL: u8 = 251;

/// Indicates that a party wishes to disable a feature on their end of the session.
pub const WONT: u8 = 252;

/// Indicates that a party wishes to enable a feature on the other end of the session.
pub const DO: u8 = 253;

/// Indicates that a party wishes to disable a feature on the other end of the session.
pub const DONT: u8 = 254;

pub mod option {
    pub const TERMINAL_TYPE: u8 = 24;
    pub const NEGO_WIN_SIZE: u8 = 31;
}

pub mod termtype {
    pub const IS: u8 = 0;
    pub const SEND: u8 = 1;
}


/// An error that may occur during a Telnet session.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    #[non_exhaustive]
    ConnectionReset { error: io::Error, opposite: SocketAddr },

    #[non_exhaustive]
    SendFailed { error: io::Error, target: SocketAddr },

    #[non_exhaustive]
    ReceiveFailed { error: io::Error, source: SocketAddr },

    #[non_exhaustive]
    UnexpectedSubNegotiationByte { byte: u8, source: SocketAddr },

    #[non_exhaustive]
    NoSubNegotiationCommand { source: SocketAddr },

    #[non_exhaustive]
    NoTerminalTypeSubNegotiationCommand { source: SocketAddr },

    #[non_exhaustive]
    UnexpectedTerminalTypeSubNegotiationCommand { byte: u8, source: SocketAddr },

    #[non_exhaustive]
    WrongWindowSizeBytes { byte_count: usize, source: SocketAddr },
}
impl Error {
    pub fn from_io_send(error: io::Error, target: SocketAddr) -> Self {
        if error.kind() == io::ErrorKind::ConnectionReset {
            Self::ConnectionReset { error, opposite: target }
        } else {
            Self::SendFailed { error, target }
        }
    }

    pub fn from_io_receive(error: io::Error, source: SocketAddr) -> Self {
        if error.kind() == io::ErrorKind::ConnectionReset {
            Self::ConnectionReset { error, opposite: source }
        } else {
            Self::ReceiveFailed { error, source }
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionReset { opposite, .. }
                => write!(f, "connection with {} reset", opposite),
            Self::SendFailed { error, target }
                => write!(f, "send to {} failed: {}", target, error),
            Self::ReceiveFailed { error, source }
                => write!(f, "receive from {} failed: {}", source, error),
            Self::UnexpectedSubNegotiationByte { byte, source }
                => write!(f, "unexpected sub-negotiaton byte 0x{:02X} from {}", byte, source),
            Self::NoSubNegotiationCommand { source }
                => write!(f, "no sub-negotiation data passed from {}", source),
            Self::NoTerminalTypeSubNegotiationCommand { source }
                => write!(f, "no terminal-type sub-negotiation data passed from {}", source),
            Self::UnexpectedTerminalTypeSubNegotiationCommand { byte, source }
                => write!(f, "unexpected terminal-type sub-negotiation byte 0x{:02X} from {}", byte, source),
            Self::WrongWindowSizeBytes { byte_count, source }
                => write!(f, "unexpected byte count {} for window size option from {}", byte_count, source),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ConnectionReset { error, .. } => Some(error),
            Self::SendFailed { error, .. } => Some(error),
            Self::ReceiveFailed { error, .. } => Some(error),
            Self::UnexpectedSubNegotiationByte { .. } => None,
            Self::NoSubNegotiationCommand { .. } => None,
            Self::NoTerminalTypeSubNegotiationCommand { .. } => None,
            Self::UnexpectedTerminalTypeSubNegotiationCommand { .. } => None,
            Self::WrongWindowSizeBytes { .. } => None,
        }
    }
}


/// Asks the client whether it can handle a "terminal type" query.
pub(crate) async fn ask_can_do_terminal_type(writer: &mut BufWriter<OwnedWriteHalf>, target: SocketAddr) -> Result<(), Error> {
    let can_term_type_ask_buf = [IAC, DO, option::TERMINAL_TYPE];
    writer.write_all(&can_term_type_ask_buf)
        .await.map_err(|e| Error::from_io_send(e, target))?;
    writer.flush()
        .await.map_err(|e| Error::from_io_send(e, target))
}

pub(crate) async fn receive_u8(reader: &mut BufReader<OwnedReadHalf>, source: SocketAddr) -> Result<u8, Error> {
    reader.read_u8()
        .await.map_err(|e| Error::from_io_receive(e, source))
}

pub(crate) async fn write_all(writer: &mut BufWriter<OwnedWriteHalf>, target: SocketAddr, buf: &[u8]) -> Result<(), Error> {
    writer.write_all(buf)
        .await.map_err(|e| Error::from_io_send(e, target))
}

pub(crate) async fn flush(writer: &mut BufWriter<OwnedWriteHalf>, target: SocketAddr) -> Result<(), Error> {
    writer.flush()
        .await.map_err(|e| Error::from_io_send(e, target))
}

pub(crate) async fn write_all_and_flush(writer: &mut BufWriter<OwnedWriteHalf>, target: SocketAddr, buf: &[u8]) -> Result<(), Error> {
    write_all(writer, target, buf).await?;
    flush(writer, target).await
}

async fn run_animation(writer: Arc<Mutex<BufWriter<OwnedWriteHalf>>>, addr: SocketAddr, config: Config) -> Result<(), Error> {
    if config.animation == "roflcopter" {
        crate::animations::roflcopter::run(writer, addr).await
    } else {
        eprintln!("unknown animation {:?} configured", config.animation);
        let mut writer_guard = writer.lock().await;
        write_all_and_flush(&mut *writer_guard, addr, b"Animation missing.").await
    }
}


pub(crate) async fn process_command<'r, 'w>(
    mut reader: &'r mut BufReader<OwnedReadHalf>,
    writer: Arc<Mutex<BufWriter<OwnedWriteHalf>>>,
    addr: SocketAddr,
    config: Config,
) -> Result<(), Error> {
    let cmd_byte = receive_u8(&mut reader, addr).await?;
    if [DO, DONT, WILL, WONT].contains(&cmd_byte) {
        // obtain feature ID
        let option_byte = receive_u8(&mut reader, addr).await?;

        match cmd_byte {
            DO => {
                // client wants us to use a feature
                match option_byte {
                    _ => {
                        eprintln!("unexpected DO option {} (0x{:02x})", option_byte, option_byte);

                        // answer with WON'T
                        let mut writer_guard = writer.lock().await;
                        write_all_and_flush(&mut *writer_guard, addr, &[IAC, WONT, option_byte]).await?;
                    }
                }
            },
            DONT => {
                // client does not want us to use a feature
                match option_byte {
                    _ => {
                        eprintln!("unexpected DON'T option {} (0x{:02x})", option_byte, option_byte);
                    },
                }
            },
            WILL => {
                // client is ready to use a feature
                match option_byte {
                    option::NEGO_WIN_SIZE => {
                        // sure, go ahead
                        let mut writer_guard = writer.lock().await;
                        write_all_and_flush(&mut *writer_guard, addr, &[IAC, DO, option_byte]).await?;
                    },
                    option::TERMINAL_TYPE => {
                        // okay, query the terminal type
                        let mut writer_guard = writer.lock().await;
                        write_all_and_flush(&mut *writer_guard, addr, &[IAC, SB, option::TERMINAL_TYPE, termtype::SEND, IAC, SE]).await?;
                    },
                    _ => {
                        eprintln!("unexpected WILL option {} (0x{:02x})", option_byte, option_byte);

                        // answer with DON'T
                        let mut writer_guard = writer.lock().await;
                        write_all_and_flush(&mut *writer_guard, addr, &[IAC, DONT, option_byte]).await?;
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
                        let config_copy = config.clone();
                        tokio::spawn(async move {
                            if let Err(e) = run_animation(writer_copy, addr, config_copy).await {
                                eprintln!("connection to {} failed: {}", addr, e);
                            }
                        });
                    },
                    _ => {
                        eprintln!("unexpected WON'T option {} (0x{:02x})", option_byte, option_byte);
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
            let b = receive_u8(&mut reader, addr).await?;
            if b == IAC {
                // read another byte
                let cmd = receive_u8(&mut reader, addr).await?;
                match cmd {
                    SE => break, // alright, it's over
                    IAC => buf.push(b), // escaped IAC
                    other => {
                        eprintln!("unexpected 0x{:02x} following IAC within subnego", other);
                        return Err(Error::UnexpectedSubNegotiationByte { byte: other, source: addr });
                    },
                }
            } else {
                buf.push(b);
            }
        }

        // okay, what do we have?
        if buf.len() == 0 {
            eprintln!("no subnego command?!");
            return Err(Error::NoSubNegotiationCommand { source: addr });
        }
        let option_byte = buf[0];
        match option_byte {
            option::TERMINAL_TYPE => {
                if buf.len() == 1 {
                    eprintln!("no termtype subnego subcomand?!");
                    return Err(Error::NoTerminalTypeSubNegotiationCommand { source: addr });
                }

                let subcommand_byte = buf[1];
                if subcommand_byte != termtype::IS {
                    eprintln!("termtype subnego subcommand is 0x{:02x}, expected 0x{:02x}", subcommand_byte, termtype::IS);
                    return Err(Error::UnexpectedTerminalTypeSubNegotiationCommand { byte: subcommand_byte, source: addr });
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
                let config_copy = config.clone();
                tokio::spawn(async move {
                    if let Err(e) = run_animation(writer_copy, addr, config_copy).await {
                        eprintln!("connection to {} failed: {}", addr, e);
                    }
                });
            },
            option::NEGO_WIN_SIZE => {
                // should be five bytes (including option)
                if buf.len() != 5 {
                    eprintln!("subnego NEGO_WIN_SIZE but buf has {} instead of 5 bytes", buf.len());
                    return Err(Error::WrongWindowSizeBytes { byte_count: buf.len(), source: addr });
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
    Ok(())
}
