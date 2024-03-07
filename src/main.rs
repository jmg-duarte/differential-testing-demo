mod simulator;

use std::{
    io::{Read, Write},
    net::TcpStream,
};

use log::{debug, error, info, trace};
use quickcheck::Arbitrary;
use simulator::Command;

use crate::simulator::Simulator;

#[derive(Debug)]
enum ResponseError {
    Error,
    Invalid,
}

// I can't re-use Result because of the foreign impl restrictions
// I can't `impl` it or `impl From` it
#[derive(Debug)]
enum Response {
    /// Execution was successful.
    Success(u8),
    /// Failed to execute the command.
    Failure(ResponseError),
}

impl From<[u8; 2]> for Response {
    fn from(value: [u8; 2]) -> Self {
        match value[0] {
            0 => Response::Success(value[1]),
            1 => Response::Failure(ResponseError::Error),
            _ => Response::Failure(ResponseError::Invalid),
        }
    }
}

/// Serialize and send a [`Command`] over the passed stream for execution.
fn execute_command(stream: &mut TcpStream, command: &Command) -> Result<Response, std::io::Error> {
    stream.write_all(&command.to_bytes())?;
    let mut buffer = [0, 0];
    stream.read_exact(&mut buffer)?;
    Ok(Response::from(buffer))
}

fn main() {
    env_logger::init();

    let mut state = Simulator::new();
    trace!("initialized simulator");

    let mut stream =
        TcpStream::connect("127.0.0.1:10203").expect("connection should be successful");
    trace!("opened connection");

    let mut g = quickcheck::Gen::new(256);
    let mut n_commands = 0;
    let mut trace: Vec<Command> = vec![];

    loop {
        let command = Command::arbitrary(&mut g);
        debug!("generated command: {:?}", command);

        let local_result = state.execute_command(&command);
        let remote_result = execute_command(&mut stream, &command);

        // Could put it before executing everything but would require a clone.
        // Pushing the command here ends up saving an allocation/free pair.
        trace.push(command);

        match (local_result, remote_result) {
            (Ok(left), Ok(Response::Success(right))) => {
                // Using match guards here would lead to the "right"
                // case falling to the match's last arm
                if left != right {
                    error!("results diverged! expected {}, received {}", left, right);
                    break;
                }
            }
            (left, Err(e)) => {
                // UnexpectedEof usually means the remote panicked
                error!("communication error: {} (local result: {:?})", e, left);
                break;
            }
            (left, Ok(right)) => {
                error!(
                    "results diverged! expected {:?}, received {:?}",
                    left, right
                );
                break;
            }
        }

        n_commands += 1;
    }

    info!("number of commands processed: {}", n_commands);
    debug!("command trace: {:?}", trace);
}
