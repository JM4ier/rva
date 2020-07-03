#![allow(dead_code, unused_variables, unused_imports)]

mod parsed;
mod parsing;
mod net;
mod netgraph;
mod link;
mod interact;

use nom;
use parsed::*;
use parsing::*;
use net::*;
use link::*;
use interact::*;

use std::collections::*;
use std::fs::File;
use std::io::prelude::*;
use walkdir::{WalkDir, DirEntry};

fn is_source_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file() && 
    entry.file_name().to_str().map(|s| s.ends_with(".rva")).unwrap_or(false)
}

fn main() {

    let mut source = String::new();

    println!("Processing files: ");
    
    for entry in WalkDir::new("."){
        let entry = entry.unwrap();
        if !is_source_file(&entry) {
            continue;
        }

        let mut file = File::open(entry.path()).unwrap();
        file.read_to_string(&mut source).unwrap();

        println!("{}", entry.path().display());
    }

    let (rest, mods) = modules(&source).unwrap();

    if rest.len() > 0 {
        println!("Warning: Not everything of the source file has been parsed:\n{}", rest);
    }

    let mut mod_map = HashMap::new();
    for m in mods.into_iter() {
        let name = m.name.to_owned();
        if let Some(_) = mod_map.insert(name.to_owned(), m) {
            panic!("Duplicate module name: {}", &name);
        }
    }

    let mut net = Net::new();
    let mut descent = HashSet::new();
    let mut wires = vec![vec![]; 3];
    let top = mod_map.get("Top").unwrap();

    let mut linker = Linker::new(top, &mut wires, &mod_map, &mut descent, &mut net).unwrap();
    let netgraph = linker.link().unwrap();

    let mut sim = Simulation::new(net);

    while !sim.is_stable() {
        sim.update();
    }

    if let Err(err) = run_interactive(&netgraph, &mut sim) {
        println!("{:?}", err);
    }
}
