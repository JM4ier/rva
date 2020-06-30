use crate::net::*;

use std::collections::*;

#[derive(PartialEq, Eq)]
pub enum WireKind {
    /// Only accessible to the local scope
    Private,
    /// Accessible to the local scope and as an input in an instance
    Input,
    /// Accessible to the local scope and as an output in an instance
    Output,
}

pub struct Wire {
    /// Name of wire
    pub name: String,

    /// number of bits this wire can hold
    pub width: usize,

    pub kind: WireKind,
}

pub struct Connection<T> {
    /// Local wire
    local: T,

    /// Instanced wire
    module: T,
}

pub struct Instance<T> {
    /// Name of the module that is being instanced
    module: String,
    /// Name that is given to this instance
    name: String,

    /// Input connections
    inputs: Vec<Connection<T>>,

    /// Output connections
    outputs: Vec<Connection<T>>,
}

pub struct Module<T> {
    /// Name of the module
    name: String,

    /// Local wires
    locals: Vec<Wire>,

    /// Local Sub-Module instances
    instances: Vec<Instance<T>>,
}

pub type ParsedModule = Module<String>;

enum WireRange {
    Ranged {from: usize, to: usize},
    Total,
}

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

