use crate::net::*;

use std::collections::*;

#[derive(PartialEq, Eq, Debug)]
pub enum WireKind {
    /// Only accessible to the local scope
    Private,
    /// Accessible to the local scope and as an input in an instance
    Input,
    /// Accessible to the local scope and as an output in an instance
    Output,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Wire {
    /// Name of wire
    pub name: String,

    /// number of bits this wire can hold
    pub width: usize,

    pub kind: WireKind,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Connection<T> {
    /// Local wires
    pub local: WireBus,

    /// Instanced wire
    pub module: T,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Instance<T> {
    /// Name of the module that is being instanced
    pub module: String,
    /// Name that is given to this instance
    pub name: String,

    /// Input connections
    pub inputs: Vec<Connection<T>>,

    /// Output connections
    pub outputs: Vec<Connection<T>>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Module<T> {
    /// Name of the module
    pub name: String,

    /// Local wires
    pub locals: Vec<Wire>,

    /// Local Sub-Module instances
    pub instances: Vec<Instance<T>>,
}

pub type ParsedModule = Module<String>;

#[derive(PartialEq, Eq, Debug)]
enum WireRange {
    Ranged {from: usize, to: usize},
    Total,
}

#[derive(PartialEq, Eq, Debug)]
pub struct WirePart {
    name: String,
    range: WireRange,
}

pub type WireBus = Vec<WirePart>;


impl ParsedModule {
    fn find_wire(&self, name: String) -> Option<(usize, &Wire)> {
        for (idx, wire) in self.locals.iter().enumerate() {
            if wire.name == name {
                return Some((idx, &wire));
            }
        }
        None
    }
}

impl WirePart {
    pub fn total(name: String) -> Self {
        Self {
            name,
            range: WireRange::Total,
        }
    }
    pub fn ranged(name: String, from: usize, to: usize) -> Self {
        Self {
            name,
            range: WireRange::Ranged {from, to},
        }
    }
}

