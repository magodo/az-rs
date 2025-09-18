use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Optional(String, Option<String>), // Can be: --enable, --enable=true, --foo bar
    Positional(String),
}

#[derive(Debug, Clone)]
pub struct CliInput {
    pub args: Vec<Arg>,
}

impl CliInput {
    pub fn new<I, S>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut result = Vec::new();
        let mut see_opt = false;
        let mut args = args.into_iter().peekable();

        while let Some(arg) = args.next() {
            if let Some(arg) = arg.as_ref().strip_prefix("-") {
                see_opt = true;
                let mut arg = arg;
                if let Some(arg2) = arg.strip_prefix("-") {
                    arg = arg2;
                }
                if let Some(eq_idx) = arg.find('=') {
                    // Handle --key=value
                    let key = arg[..eq_idx].to_string();
                    let value = Some(arg[eq_idx + 1..].to_string());
                    result.push(Arg::Optional(key, value));
                } else {
                    // Handle --key [value] or just --flag
                    let key = arg.to_string();
                    match args.peek() {
                        Some(next) if !next.as_ref().starts_with("-") => {
                            let value = args.next().map(|v| String::from(v.as_ref()));
                            result.push(Arg::Optional(key, value));
                        }
                        _ => {
                            // It's a flag with no value
                            result.push(Arg::Optional(key, None));
                        }
                    }
                }
            } else {
                if see_opt {
                    anyhow::bail!("optional raw arguments must follow positional raw arguments");
                }
                // Positional argument
                result.push(Arg::Positional(String::from(arg.as_ref())));
            }
        }
        Ok(Self { args: result })
    }

    pub fn is_help(&self) -> bool {
        self.args
            .iter()
            .filter(|arg| {
                if let Arg::Optional(k, _) = arg {
                    return k == "h" || k == "help";
                }
                false
            })
            .count()
            != 0
    }

    pub fn api_version(&self) -> Option<String> {
        if let Arg::Optional(_, v) = self.args.iter().find(|arg| {
            if let Arg::Optional(k, _) = arg {
                k == "api-version"
            } else {
                false
            }
        })? {
            v.clone()
        } else {
            None
        }
    }

    pub fn pos_args(&self) -> Vec<&str> {
        self.args
            .iter()
            .filter_map(|arg| {
                if let Arg::Positional(arg) = arg {
                    Some(arg.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.args.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod test {
    use crate::arg::{Arg, CliInput};

    #[test]
    fn new_cli_input() {
        assert_eq!(
            CliInput::new(vec!["foo"]).unwrap().args,
            vec![Arg::Positional(String::from("foo"))]
        );
        assert_eq!(
            CliInput::new(vec!["foo", "bar"]).unwrap().args,
            vec![
                Arg::Positional(String::from("foo")),
                Arg::Positional(String::from("bar")),
            ]
        );
        assert_eq!(
            CliInput::new(vec!["foo", "--bar"]).unwrap().args,
            vec![
                Arg::Positional(String::from("foo")),
                Arg::Optional(String::from("bar"), None),
            ]
        );
        assert_eq!(
            CliInput::new(vec!["foo", "-b"]).unwrap().args,
            vec![
                Arg::Positional(String::from("foo")),
                Arg::Optional(String::from("b"), None),
            ]
        );
        assert_eq!(
            CliInput::new(vec!["foo", "--bar=baz"]).unwrap().args,
            vec![
                Arg::Positional(String::from("foo")),
                Arg::Optional(String::from("bar"), Some(String::from("baz"))),
            ]
        );
        assert_eq!(
            CliInput::new(vec!["foo", "-b=baz"]).unwrap().args,
            vec![
                Arg::Positional(String::from("foo")),
                Arg::Optional(String::from("b"), Some(String::from("baz"))),
            ]
        );
        assert_eq!(
            CliInput::new(vec!["foo", "--bar", "baz"]).unwrap().args,
            vec![
                Arg::Positional(String::from("foo")),
                Arg::Optional(String::from("bar"), Some(String::from("baz"))),
            ]
        );
        assert_eq!(
            CliInput::new(vec!["foo", "-b", "baz"]).unwrap().args,
            vec![
                Arg::Positional(String::from("foo")),
                Arg::Optional(String::from("b"), Some(String::from("baz"))),
            ]
        );
    }
}
