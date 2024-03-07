use quickcheck::Arbitrary;

/// Simulator errors.
#[derive(Debug)]
pub enum Error {
    /// Read out of bounds.
    InvalidRead,
    /// Write out of bounds.
    InvalidWrite,
    /// Integer overflow.
    Overflow,
}

/// Simulator for the remote to detect discrepancies.
pub struct Simulator([u8; 4]);

impl Simulator {
    /// Create a new [`Simulator`].
    pub fn new() -> Self {
        Self([0; 4])
    }

    /// Execute a [`Command`] on the [`Simulator`].
    pub fn execute_command(&mut self, command: &Command) -> Result<u8, Error> {
        match command {
            Command::Read(location) => self
                .0
                // SAFETY: usize is bigger than u8
                .get(*location as usize)
                .copied()
                .ok_or(Error::InvalidRead),
            Command::Write(location, value) => {
                // SAFETY: usize is bigger than u8
                if let Some(stored_value) = self.0.get_mut(*location as usize) {
                    *stored_value = *value;
                    Ok(*value)
                } else {
                    Err(Error::InvalidWrite)
                }
            }
            // `sum` and `product` don't check for overflow, they'll panic in debug and wrap in release
            // thus, here I do a checked_X to detect issues with overflow
            Command::Sum => {
                // .sum doesn't check for overflow
                let mut acc: u8 = 0;
                for v in &self.0 {
                    if let Some(res) = acc.checked_add(*v) {
                        acc = res;
                    } else {
                        return Err(Error::Overflow);
                    }
                }
                Ok(acc)
            }
            Command::Product => {
                let mut acc: u8 = 1;
                for v in &self.0 {
                    if let Some(res) = acc.checked_mul(*v) {
                        acc = res;
                    } else {
                        return Err(Error::Overflow);
                    }
                }
                Ok(acc)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    /// Read a byte at the given index.
    Read(u8),
    /// Write a byte (2nd value) at the given index (1st value).
    Write(u8, u8),
    /// Sum all values in memory.
    Sum,
    /// Multiply all values in memory.
    Product,
}

impl Command {
    pub fn to_bytes(&self) -> [u8; 3] {
        match self {
            Command::Read(location) => [1, *location, 0],
            Command::Write(location, value) => [2, *location, *value],
            Command::Sum => [3, 0, 0],
            Command::Product => [4, 0, 0],
        }
    }
}

impl Arbitrary for Command {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        // Choosing 0 to 3 since 4 will panic the remote, we could add that but leads to not very interesting tests
        // Typing is easier this way, hence the static
        static CHOICES: [u8; 5] = [0, 1, 2, 3, 4];
        fn read(g: &mut quickcheck::Gen) -> Command {
            Command::Read(*(g.choose(&CHOICES).unwrap()))
        }
        fn write(g: &mut quickcheck::Gen) -> Command {
            Command::Write(*(g.choose(&CHOICES).unwrap()), u8::arbitrary(g))
        }
        fn product(_: &mut quickcheck::Gen) -> Command {
            Command::Product
        }
        fn sum(_: &mut quickcheck::Gen) -> Command {
            Command::Sum
        }
        // This is a weird dialect at first sight but allows for lazyness when generating the cases
        // furthermore, it's simpler when picking a branch because they're built on demand
        g.choose(&[
            read as fn(&mut quickcheck::Gen) -> Command,
            write as fn(&mut quickcheck::Gen) -> Command,
            product as fn(&mut quickcheck::Gen) -> Command,
            sum as fn(&mut quickcheck::Gen) -> Command,
        ])
        // SAFETY: `choose` docs state that this will never be none if a non-empty slice is passed
        .unwrap()(g)
    }
}
