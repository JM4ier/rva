#![allow(dead_code, unused_variables, unused_imports)]

mod parsed;
mod parsing;
mod net;
mod netgraph;
mod link;

use nom;
use parsed::*;
use parsing::*;
use net::*;
use link::*;

use std::collections::*;

const TEST_FILE: &'static str = 
"
module not (in) -> (out) {
    nor inv(a=in, b=in) -> (out);
}

module and(a, b) -> (out) {
    wire inva, invb;

    not not_a(in=a) -> (out=inva);
    not not_b(in=b) -> (out=invb);

    nor nor(a=inva, b=invb) -> (out);
}

module or(a, b) -> (out) {
    wire norab;
    nor ng(a, b) -> (out=norab);
    not inv(in=norab) -> (out);
}

module nand(a, b) -> (out) {
    wire andab;
    and and_(a, b) -> (out=andab);
    not inv(in=andab) -> (out);
}

module xnor(a, b) -> (out) {
    wire a_and_b, nor_a_b;
    and and(a, b) -> (out=a_and_b);
    nor nor(a, b) -> (out=nor_a_b);
    or or(a=a_and_b, b=nor_a_b) -> (out);
}

module xor(a, b) -> (out) {
    wire a_xnor_b;
    xnor xnor(a, b) -> (out=a_xnor_b);
    not not(in=a_xnor_b) -> (out);
}

module SRLatch(s, r) -> (q) {
    wire notq;
    nor nora(a=r, b=notq) -> (out=q);
    nor norb(a=s, b=q) -> (out=notq);
}

module DLatch(clk, d) -> (q) {
    wire s, r, invd;

    not not_d(in=d) -> (out=invd);

    and ands(a=d, b=clk) -> (out=s);
    and andr(a=invd, b=clk) -> (out=r);

    SRLatch sr(s=s, r=r) -> (q=q);
}

module DFlipFlop(clk, d) -> (q) {
    wire invclk;
    wire slaved;

    not not_clk(in=clk) -> (out=invclk);

    DLatch master(clk=invclk, d=d) -> (q=slaved);
    DLatch slave(clk, d=slaved) -> (q=q);
}

module Top() -> () {
    wire a, b, c;
    DFlipFlop some_gate_name(clk=a, d=b) -> (q=c);
}
";

fn main() {
    let nor = module("module nor(a, b) -> (out) {}").unwrap().1;
    let (rest, mut mods) = modules(TEST_FILE).unwrap();

    mods.push(nor);

    println!("{:#?}", mods);
    println!("{:#?}", rest);

    let top = "Top";

    let mut mod_map = HashMap::new();
    for m in mods.into_iter() {
        let name = m.name.to_owned();
        if let Some(_) = mod_map.insert(name.to_owned(), m) {
            panic!("Duplicate module name: {}", &name);
        }
    }

    let mut net = Net::new();
    let mut heritage = HashSet::new();
    let mut wires = vec![vec![]; 3];
    let top = mod_map.get(top).unwrap();

    let mut linker = Linker::new(top, &mut wires, &mod_map, &mut heritage, &mut net).unwrap();
    let netgraph = linker.link().unwrap();


    let mut sim = Simulation::new(net);

    while !sim.is_stable() {
        sim.update();
    }

    println!("{:#?}", sim);
    println!("{:#?}", netgraph);
}
