#![deny(clippy::all)]

use clap::{crate_version, value_parser, Arg, ArgAction, Command};
use sbnf::formatter::{format, FormatterOptions};
use sbnf::sbnf::lexer::lex;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    std::process::exit(match try_main() {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{}", e);

            1
        }
    });
}

fn fmt_io_err<T>(r: std::io::Result<T>) -> Result<T, String> {
    r.map_err(|e| format!("{}", e))
}

fn read_input(input: &str) -> Result<String, String> {
    let mut contents = String::new();

    if input == "-" {
        fmt_io_err(std::io::stdin().read_to_string(&mut contents))?;
    } else {
        let mut file = fmt_io_err(File::open(input))?;
        fmt_io_err(file.read_to_string(&mut contents))?;
    }

    Ok(contents)
}

fn try_main() -> Result<(), String> {
    let matches = Command::new("SBNF formatter")
        .version(crate_version!())
        .about("Format SBNF grammars")
        .arg(
            Arg::new("check")
                .short('c')
                .long("check")
                .action(ArgAction::SetTrue)
                .help("Check if the input is already formatted"),
        )
        .arg(
            Arg::new("write")
                .short('w')
                .long("write")
                .action(ArgAction::SetTrue)
                .conflicts_with("check")
                .help("Write the formatted output back to the input file"),
        )
        .arg(
            Arg::new("no_reorder")
                .long("no-reorder")
                .action(ArgAction::SetTrue)
                .help("Do not reorder declarations"),
        )
        .arg(
            Arg::new("max_width")
                .long("max-width")
                .value_name("WIDTH")
                .value_parser(value_parser!(usize))
                .default_value("80")
                .help("Maximum line length"),
        )
        .arg(
            Arg::new("INPUT")
                .required(true)
                .help("The SBNF file to format, or - for stdin"),
        )
        .get_matches();

    let input = matches.get_one::<String>("INPUT").unwrap();
    let check = matches.get_flag("check");
    let write = matches.get_flag("write");
    let reorder = !matches.get_flag("no_reorder");
    let max_line_length = *matches.get_one::<usize>("max_width").unwrap();

    let contents = read_input(input)?;
    let grammar = lex(&contents);
    let options = FormatterOptions {
        reorder,
        ensure_trailing_newline: true,
        max_line_length,
    };
    let formatted = format(options, &grammar.tokens);

    if check {
        if formatted != contents {
            return Err(format!("{} is not formatted", input));
        }

        return Ok(());
    }

    if write {
        if input == "-" {
            return Err("Cannot use --write with stdin".to_string());
        }

        if formatted != contents {
            let mut file = fmt_io_err(File::create(input))?;
            fmt_io_err(file.write_all(formatted.as_bytes()))?;
        }

        return Ok(());
    }

    fmt_io_err(std::io::stdout().write_all(formatted.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(source: &str) -> String {
        let grammar = lex(source);
        format(FormatterOptions::default(), &grammar.tokens)
    }

    #[test]
    fn format_source() {
        assert_eq!(fmt("foo:bar;"), "foo : bar ;\n");
    }
}
