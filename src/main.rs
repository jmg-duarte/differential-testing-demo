use std::{
    io::{Read, Write},
    net::TcpStream,
};

use log::{debug, error, info};
use quickcheck::Arbitrary;

#[derive(Debug)]
enum ExecutionError {
    InvalidRead,
    InvalidWrite,
    Overflow,
}

struct State([u8; 4]);

impl State {
    fn new() -> Self {
        Self([0; 4])
    }

    fn execute_command(&mut self, command: &Command) -> Result<u8, ExecutionError> {
        match command {
            Command::Read(location) => self
                .0
                // SAFETY: usize is bigger than u8
                .get(*location as usize)
                .copied()
                .ok_or(ExecutionError::InvalidRead),
            Command::Write(location, value) => {
                // SAFETY: usize is bigger than u8
                if let Some(stored_value) = self.0.get_mut(*location as usize) {
                    *stored_value = *value;
                    Ok(*value)
                } else {
                    Err(ExecutionError::InvalidWrite)
                }
            }
            Command::Sum => {
                // .sum doesn't check for overflow
                let mut acc: u8 = 0;
                for v in self.0.iter() {
                    if let Some(res) = acc.checked_add(*v) {
                        acc = res
                    } else {
                        return Err(ExecutionError::Overflow);
                    }
                }
                Ok(acc)
            }
            Command::Product => {
                // .product doesn't check for overflow
                let mut acc: u8 = 1;
                for v in self.0.iter() {
                    if let Some(res) = acc.checked_mul(*v) {
                        acc = res
                    } else {
                        return Err(ExecutionError::Overflow);
                    }
                }
                Ok(acc)
            }
        }
    }

    // fn execute_commands<I: IntoIterator<Item = Command>>(
    //     &mut self,
    //     commands: I,
    // ) -> Result<u8, ExecutionError> {
    //     let mut last_result = 0;
    //     for command in commands {
    //         last_result = self.execute_command(&command)?;
    //     }
    //     Ok(last_result)
    // }
}

#[derive(Clone, Debug)]
enum Command {
    Read(u8),
    Write(u8, u8),
    Sum,
    Product,
}

impl Command {
    fn to_bytes(&self) -> [u8; 3] {
        match self {
            Command::Read(location) => [1, *location, 0],
            Command::Write(location, value) => [2, *location, *value],
            Command::Sum => [3, 0, 0],
            Command::Product => [4, 0, 0],
        }
    }
}

#[derive(Debug)]
enum ReadError {
    Error,
    Invalid,
}

// I can't re-use Result because of the foreign impl restrictions
#[derive(Debug)]
enum Response {
    Success(u8),
    Failure(ReadError),
}

impl From<[u8; 2]> for Response {
    fn from(value: [u8; 2]) -> Self {
        match value[0] {
            0 => Response::Success(value[1]),
            1 => Response::Failure(ReadError::Error),
            _ => Response::Failure(ReadError::Invalid),
        }
    }
}

fn execute_command(stream: &mut TcpStream, command: &Command) -> Result<Response, std::io::Error> {
    stream.write_all(&command.to_bytes())?;
    let mut buffer = [0, 0];
    stream.read_exact(&mut buffer)?;
    Ok(Response::from(buffer))
}

// TODO: sweep_read — read all 4 state bytes to check for values
// TODO: assert over sweep_read

impl Arbitrary for Command {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        // Choosing 0 to 3 since 4 will panic the remote
        // Typing is easier this way, hence the static
        static CHOICES: [u8; 4] = [0, 1, 2, 3];
        fn read(g: &mut quickcheck::Gen) -> Command {
            let ret = Command::Read(*(g.choose(&CHOICES).unwrap()));
            ret
        }
        fn write(g: &mut quickcheck::Gen) -> Command {
            let ret = Command::Write(*(g.choose(&CHOICES).unwrap()), u8::arbitrary(g));
            ret
        }
        fn product(_: &mut quickcheck::Gen) -> Command {
            let ret = Command::Product;
            ret
        }
        fn sum(_: &mut quickcheck::Gen) -> Command {
            let ret = Command::Sum;
            ret
        }
        // This is a weird dialect but allows for lazyness when generating the cases
        g.choose(&[
            read as fn(&mut quickcheck::Gen) -> Command,
            write as fn(&mut quickcheck::Gen) -> Command,
            product as fn(&mut quickcheck::Gen) -> Command,
            sum as fn(&mut quickcheck::Gen) -> Command,
        ])
        .unwrap()(g)
    }
}

fn main() {
    env_logger::init();

    let mut state = State::new();

    let mut stream =
        TcpStream::connect("127.0.0.1:10203").expect("connection should be successful");

    let mut g = quickcheck::Gen::new(256);
    let mut n_commands = 0;
    loop {
        let command = Command::arbitrary(&mut g);
        debug!("generated command: {:?}", command);

        let local_result = state.execute_command(&command);
        let remote_result = execute_command(&mut stream, &command);

        match (local_result, remote_result) {
            // TODO add better reporting
            (Ok(left), Ok(Response::Success(right))) => {
                if left != right {
                    error!("results diverged! expected {}, received {}", left, right);
                    info!("number of commands processed: {}", n_commands);
                    return;
                }
            }
            (left, Err(e)) => {
                error!("local result: {:?}", left);
                error!("remote communication error: {}", e);
                info!("number of commands processed: {}", n_commands);
                return;
            }
            (left, Ok(right)) => {
                error!(
                    "results diverged! expected {:?}, received {:?}",
                    left, right
                );
                info!("number of commands processed: {}", n_commands);

                return;
            }
        }

        n_commands += 1;
    }
}
