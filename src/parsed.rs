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
    pub name: String,

    /// number of bits this wire can hold
    pub width: usize,

    pub kind: WireKind,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Connection {
    /// Local wires
    pub local: WireBus,

    /// Instanced wire name
    pub module: String,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Instance {
    pub module: String,
    pub name: String,
    pub inputs: Vec<Connection>,
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

