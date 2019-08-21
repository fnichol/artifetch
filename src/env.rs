use std::env;
use std::error;
use std::fmt;

pub fn replace_vars(s: &mut String) -> Result<(), Error> {
    enum State {
        No,
        Found(usize),
        InBrace(usize),
        InNoBrace(usize),
        Brace(usize, usize),
        NoBrace(usize, usize),
    }

    fn valid_var_char(ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_'
    }

    use State::*;

    loop {
        let mut state = No;

        for (idx, ch) in s.char_indices() {
            match state {
                No if ch == '$' => {
                    state = Found(idx);
                }
                Found(start_idx) => {
                    if ch == '{' {
                        state = InBrace(start_idx);
                    } else if valid_var_char(ch) {
                        state = InNoBrace(start_idx);
                    } else {
                        state = No;
                    }
                }
                InBrace(start_idx) => {
                    if ch == '}' {
                        state = Brace(start_idx, idx);
                        break;
                    } else if !valid_var_char(ch) {
                        return Err(Error::InvalidBraceChar(idx, ch));
                    }
                }
                InNoBrace(start_idx) => {
                    if !valid_var_char(ch) {
                        state = NoBrace(start_idx, idx);
                        break;
                    }
                }
                _ => {}
            }
        }

        // If we're looking for the end of a variable but we hit the end of the string
        match state {
            // Finalize the non-braced variable
            InNoBrace(start_idx) => {
                state = NoBrace(start_idx, s.len());
            }
            // Return an error as the braced variable did not terminate properly
            InBrace(start_idx) => {
                return Err(Error::NonTerminatedBrace(start_idx));
            }
            _ => {}
        }

        match state {
            // Replace the braced variable with its environment variable value
            Brace(start_idx, end_idx) => {
                let name = s.get((start_idx + 2)..end_idx).expect("range should exist");
                let val =
                    env::var(&name).map_err(|err| Error::EnvVarNotFound(name.to_string(), err))?;
                s.replace_range(start_idx..=end_idx, &val);
            }
            // Replace the not braced variable with its environment variable value
            NoBrace(start_idx, end_idx) => {
                let name = s.get((start_idx + 1)..end_idx).expect("range should exist");
                let val =
                    env::var(&name).map_err(|err| Error::EnvVarNotFound(name.to_string(), err))?;
                s.replace_range(start_idx..end_idx, &val);
            }
            // If no variables were found, then terminate the re-scan loop
            No => {
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    EnvVarNotFound(String, env::VarError),
    InvalidBraceChar(usize, char),
    NonTerminatedBrace(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::EnvVarNotFound(ref name, _) => {
                write!(f, "environment variable not set; name={}", name)
            }
            Error::InvalidBraceChar(ref idx, ref ch) => write!(
                f,
                "invalid char found in braced variable name; char={}, idx={}",
                ch, idx
            ),
            Error::NonTerminatedBrace(ref idx) => write!(
                f,
                "braced variable not properly terminated; starting_idx={}",
                idx
            ),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::EnvVarNotFound(_, ref err) => err.source(),
            Error::InvalidBraceChar(_, _) => None,
            Error::NonTerminatedBrace(_) => None,
        }
    }
}
