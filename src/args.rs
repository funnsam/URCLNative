use crate::VERSION;
use ParseResult::*;
pub enum ParseResult {
    Ok(Args),
    Err(String),
    Help(String)
}

pub struct Args {
    pub file    : String,
    pub output  : String,
    pub debug   : bool
}

impl Args {
    pub fn parse() -> ParseResult {
        let args = std::env::args().collect::<Vec<String>>();
        if args.len() < 2 {
            return Err("Not enough arguments, please check out \"urcln --help\"".to_string());
        };

        match args[1].as_str() {
            "-h" | "--help" => return Help (
                "\x1b[1;4mUsage:\x1b[0m\n".to_string() +
                "    \x1b[1;34murcln\x1b[0;32m <Input file> <Options>\x1b[0m\n" +
                "\x1b[1;4mOptions:\x1b[0m\n" +
                "    \x1b[33m-o, --output\x1b[0;32m <File>\x1b[0m      Sets output file \x1b[90m(Default: \"a.out\")\x1b[0m\n" +
                "    \x1b[33m-g, --debug\x1b[0m              Enable debug infomation"
            ),
            "-v" | "--version" => return Help (
                format!("URCLNative version {VERSION}\n") +
                "    \x1b[4;34mhttps://github.com/funnsam/URCLNative/blob/LICENSE\x1b[0m"
            ),
            _ => ()
        }

        let mut a = Self {
            file    : args[1].clone(),
            output  : "a.o".to_string(),
            debug   : false
        };
        let mut stat = ParserStatus::None;

        for i in args[2..].iter() {
            match stat {
                ParserStatus::Output => {
                    a.output = i.clone();
                    stat = ParserStatus::None;
                },
                ParserStatus::None => {
                    match i.as_str() {
                        "-o" | "--output"
                            => stat = ParserStatus::Output,
                        "-g" | "--debug"
                            => a.debug = true,
                        _
                            => return Err(format!("Unexpected \"{i}\""))
                    }
                }
            }
        }

        Ok(a)
    }
}

enum ParserStatus {
    None, Output
}
