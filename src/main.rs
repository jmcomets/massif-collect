use std::fs::File;
use std::io::{self, BufReader};

use massif_collect::{read_massif, ui};

use clap::{App, Arg};

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
        .arg(Arg::with_name("snapshot")
             .help("Sets the input snapshot to view")
             .index(1))
}

fn main() -> io::Result<()> {
    let matches = app().get_matches();
    let filename = matches.value_of("snapshot").unwrap_or("data/example.out");
    let ui_stdout = matches.value_of("tui-stdout");

    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let (caller_tree, call_graph) = read_massif(reader)?;

    ui::run(ui_stdout, &caller_tree, &call_graph)?;

    Ok(())
}
