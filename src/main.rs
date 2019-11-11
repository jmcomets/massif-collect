#![allow(dead_code, unused_imports, unused_variables)] // FIXME remove this

use std::fs::File;
use std::io::{self, Read, BufRead, BufReader};

use massif_collect::{parsing, ui};

use clap::{App, Arg};

macro_rules! io_error {
    ($tag:expr) => {{
        |e| {
            let message = format!("{}: {:?}", $tag, e);
            ::std::io::Error::new(::std::io::ErrorKind::Other, message)
        }
    }}
}

fn app() -> App<'static, 'static> {
    App::new("massif-collect")
        .version("0.1")
        .author("Jean-Marie Comets <jean.marie.comets@gmail.com>")
        .about("Analyze Massif snapshots")
        .arg(Arg::with_name("tui-stdout")
             .long("tui-stdout")
             .value_name("TTY")
             .help("Set the text user-interface's standard output.")
             .takes_value(true))
        .arg(Arg::with_name("out")
             .help("Massif output file to view")
             .index(1))
}

fn main() -> io::Result<()> {
    let matches = app().get_matches();
    let filename = matches.value_of("out").unwrap_or("data/example.out");
    let ui_stdout = matches.value_of("tui-stdout");

    eprint!("Reading file into memory ... ");
    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);
    let mut input = String::new();
    reader.read_to_string(&mut input)?;
    eprintln!("done");

    eprint!("Parsing ... ");
    let (_, (_, snapshots)) = parsing::massif(&input)
        .map_err(io_error!("reading massif output"))?;
    eprintln!("done");

    ui::run(ui_stdout, &snapshots[..])?;

    Ok(())
}
