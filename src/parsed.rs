#[derive(PartialEq, Eq, Debug, Copy, Clone)]
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

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum WireRange {
    Ranged {from: usize, to: usize},
    Total,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum WirePart {
    Local{name: String, range: WireRange},
    Constant(Vec<bool>),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct WireAssignment {
    pub bus: WireBus,
    pub operation: Operation,
}

type Op = Box<Operation>;
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Operation {
    Wire(WireBus),
    And(Op, Op),
    Or(Op, Op),
    Xor(Op, Op),
    AndReduce(Op),
    OrReduce(Op),
    XorReduce(Op),
    Not(Op),
}

pub type WireBus = Vec<WirePart>;

impl WirePart {
    pub fn total<T: ToString>(name: T) -> Self {
        WirePart::Local {
            name: name.to_string(),
            range: WireRange::Total,
        }
    }
    pub fn ranged<T: ToString>(name: T, from: usize, to: usize) -> Self {
        Self::Local {
            name: name.to_string(),
            range: WireRange::Ranged {from, to},
        }
    }
    pub fn constant(constant: Vec<bool>) -> Self {
        Self::Constant(constant)
    }
    pub fn width(&self, module: &Module) -> Result<usize, ()> {
        match self {
            Self::Constant(c) => Ok(c.len()),
            Self::Local{name, range} => {
                match range {
                    WireRange::Ranged{from, to} => Ok(to-from+1),
                    WireRange::Total => {
                        for local in module.locals.iter() {
                            if local.name == *name {
                                return Ok(local.width);
                            }
                        }
                        Err(())
                    }
                }
            }
        }
    }
}

impl Operation {
    pub fn width(&self, module: &Module) -> Result<usize, ()> {
        match self {
            Self::Wire(bus) => bus.iter().map(|w| w.width(module)).sum(),
            Self::And(op1, op2) => op1.width(module).max(op2.width(module)),
            Self::Xor(op1, op2) => op1.width(module).max(op2.width(module)),
            Self::Or(op1, op2)  => op1.width(module).max(op2.width(module)),
            Self::Not(op) => op.width(module),
            _ => Ok(1), // reductions/reduces? lead to a 1-bit result
        }
    }
}

