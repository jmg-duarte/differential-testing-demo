mod simulator;

use std::{
    io::{Read, Write},
    net::TcpStream,
};

use env_logger::Env;
use log::{debug, error, info, trace};
use quickcheck::Arbitrary;
use simulator::Command;

use crate::simulator::Simulator;

/// Response error.
#[derive(Debug)]
enum ResponseError {
    /// Response received was an error.
    Error,
    /// Invalid response received, this means the first byte was neither 0 nor 1.
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
fn execute_command_on_stream(
    stream: &mut TcpStream,
    command: &Command,
) -> Result<Response, std::io::Error> {
    stream.write_all(&command.to_bytes())?;
    let mut buffer = [0, 0];
    stream.read_exact(&mut buffer)?;
    Ok(Response::from(buffer))
}

/// Execute a command on both the simulator and and the stream, checking for divergences.
fn execute_command(
    simulator: &mut Simulator,
    stream: &mut TcpStream,
    command: &Command,
) -> Result<(), ()> {
    // This return value could be better, but since the errors are reported in this function too
    // I think it is clearer to use this type instead of a `bool`, as at the return site
    // `.is_err` clearly indicates whether or not an error occurred while `true`/`false` isn't as clear
    let local_result = simulator.execute_command(command);
    let remote_result = execute_command_on_stream(stream, command);

    match (local_result, remote_result) {
        (Ok(left), Ok(Response::Success(right))) => {
            // Using match guards here would lead to the "right"
            // case falling to the match's last arm
            if left != right {
                error!("results diverged! expected {}, received {}", left, right);
                return Err(());
            }
        }
        (left, Err(e)) => {
            // UnexpectedEof usually means the remote panicked
            error!("communication error: {} (local result: {:?})", e, left);
            return Err(());
        }
        (left, Ok(right)) => {
            error!(
                "results diverged! expected {:?}, received {:?}",
                left, right
            );
            return Err(());
        }
    }

    Ok(())
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let mut simulator = Simulator::new();
    trace!("initialized simulator");

    let mut stream =
        TcpStream::connect("127.0.0.1:10203").expect("connection should be successful");
    trace!("opened connection");

    let mut g = quickcheck::Gen::new(256);
    let mut trace: Vec<Command> = vec![];

    loop {
        let command = Command::arbitrary(&mut g);
        debug!("generated command: {:?}", command);
        trace.push(command);

        let command = trace
            .last()
            .expect("the command was just pushed, it should be in the vector's last position");

        if execute_command(&mut simulator, &mut stream, command).is_err() {
            break;
        }
    }

    info!("number of commands processed: {}", trace.len());
    debug!("command trace: {:?}", trace);
}
