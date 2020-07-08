mod parsed;
mod parsing;
mod assignment;
mod net;
mod netgraph;
mod link;
mod interact;

use parsed::*;
use parsing::*;
use net::*;
use netgraph::*;
use link::*;
pub use interact::*;

use std::collections::*;
use std::fs::File;
use std::io::prelude::*;
use walkdir::{WalkDir, DirEntry};

fn is_source_file(entry: &DirEntry) -> bool {
    entry.file_type().is_file() && 
    entry.file_name().to_str().map(|s| s.ends_with(".rva")).unwrap_or(false)
}

fn read_source() -> String {
    let mut source = String::new();

    println!("Reading files: ");
    for entry in WalkDir::new("."){
        let entry = entry.unwrap();
        if !is_source_file(&entry) {
            continue;
        }

        let mut file = File::open(entry.path()).unwrap();
        file.read_to_string(&mut source).unwrap();

        println!("{}", entry.path().display());
    }
    println!();
    source
}

fn parse(source: &String) -> Vec<Module> {
    let (rest, mods) = modules(&source).unwrap();
    if rest.len() > 0 {
        eprintln!("Warning: Not everything of the source file has been parsed:\n{}", rest);
    }
    mods
}

fn build(mods: Vec<Module>) -> LinkResult<(GraphModule, Simulation)> {
    let mut mod_map = HashMap::new();
    for m in mods.into_iter() {
        let name = m.name.to_owned();
        if let Some(_) = mod_map.insert(name.to_owned(), m) {
            panic!("Duplicate module name: {}", &name);
        }
    }

    let mut net = Net::new();
    let mut descent = Vec::new();
    let mut wires = vec![vec![]; 3];
    let top = mod_map.get("Top").expect("No 'Top' Module found");

    let mut linker = Linker::new(top, &mut wires, &mod_map, &mut descent, &mut net)?;
    let graph = linker.link()?;

    let sim = Simulation::new(net);

    Ok((graph, sim))
}

#[repr(C)]
pub struct GraphAndSimulation {
    graph: *mut GraphModule,
    sim: *mut Simulation,
}

#[no_mangle]
pub extern "C" fn create_graph_simulation() -> GraphAndSimulation {
    let source = read_source();
    let mods = parse(&source);

    match build(mods) {
        Ok((graph, sim)) => {
            let graph = Box::into_raw(Box::new(graph));
            let sim   = Box::into_raw(Box::new(sim));

            let ptr = GraphAndSimulation {
                graph, sim
            };

            std::mem::forget(graph);
            std::mem::forget(sim);

            ptr
        },
        Err(e) => {
            eprintln!("Failed to link modules ({:?}): {}", e.kind, e.description);
            std::process::exit(1);
        },
    }
}

