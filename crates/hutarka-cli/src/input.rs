use std::io::Read;

#[derive(Debug, PartialEq, Eq)]
pub enum InputError {
    NoInput,
    Empty,
    Read(String),
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputError::NoInput => {
                write!(f, "no input: pass text as an argument or pipe it on stdin")
            }
            InputError::Empty => write!(f, "input is empty"),
            InputError::Read(e) => write!(f, "cannot read stdin: {e}"),
        }
    }
}

/// The text to send: the argument if given, else all of stdin. Never waits on
/// a terminal. Returned unchanged (not trimmed).
pub fn read_input(
    arg: Option<String>,
    stdin_is_tty: bool,
    mut stdin: impl Read,
) -> Result<String, InputError> {
    let text = match arg {
        Some(text) => text,
        None if stdin_is_tty => return Err(InputError::NoInput),
        None => {
            let mut buf = String::new();
            stdin
                .read_to_string(&mut buf)
                .map_err(|e| InputError::Read(e.to_string()))?;
            buf
        }
    };
    if text.trim().is_empty() {
        return Err(InputError::Empty);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    struct MustNotRead;
    impl Read for MustNotRead {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            panic!("stdin must not be read");
        }
    }

    #[test]
    fn argument_wins_over_stdin() {
        assert_eq!(
            read_input(Some("arg".into()), false, MustNotRead),
            Ok("arg".into())
        );
    }

    #[test]
    fn reads_stdin_when_no_argument() {
        assert_eq!(
            read_input(None, false, "a\nb\n".as_bytes()),
            Ok("a\nb\n".into())
        );
    }

    #[test]
    fn tty_stdin_without_argument_is_an_error() {
        assert_eq!(
            read_input(None, true, MustNotRead),
            Err(InputError::NoInput)
        );
    }

    #[test]
    fn whitespace_only_is_empty() {
        assert_eq!(
            read_input(None, false, " \n\t".as_bytes()),
            Err(InputError::Empty)
        );
        assert_eq!(
            read_input(Some("  ".into()), false, MustNotRead),
            Err(InputError::Empty)
        );
    }
}
