use std::env;
use std::fs::File;
use std::io::{self, BufReader};

use massif_collect::{read_massif, ui};

fn main() -> io::Result<()> {
    let filename = env::args().nth(1).unwrap_or("data/example.out".to_string());
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let (caller_tree, call_graph) = read_massif(reader)?;

    ui::run(&caller_tree, &call_graph)?;

    Ok(())
}
