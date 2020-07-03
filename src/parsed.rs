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
pub struct Connection {
    /// Local wires
    pub local: WireBus,

    /// Instanced wire
    pub module: String,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Instance {
    /// Name of the module that is being instanced
    pub module: String,
    /// Name that is given to this instance
    pub name: String,

    /// Input connections
    pub inputs: Vec<Connection>,

    /// Output connections
    pub outputs: Vec<Connection>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Module {
    /// Name of the module
    pub name: String,

    /// Local wires
    pub locals: Vec<Wire>,

    /// Local Sub-Module instances
    pub instances: Vec<Instance>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum WireRange {
    Ranged {from: usize, to: usize},
    Total,
}

#[derive(PartialEq, Eq, Debug)]
pub enum WirePart {
    Local{name: String, range: WireRange},
    Constant(Vec<bool>),
}

pub type WireBus = Vec<WirePart>;

impl WirePart {
    pub fn total(name: String) -> Self {
        Self::Local {
            name,
            range: WireRange::Total,
        }
    }
    pub fn ranged(name: String, from: usize, to: usize) -> Self {
        Self::Local {
            name,
            range: WireRange::Ranged {from, to},
        }
    }
    pub fn constant(constant: Vec<bool>) -> Self {
        Self::Constant(constant)
    }
}

