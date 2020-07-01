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
pub type LinkedModule = Module<Vec<usize>>;

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

#[derive(Debug)]
pub enum LinkError {
    Recursion,
    WireMismatch,
    MismatchedWireSize(String),
    DuplicateWireName(String),
    MissingIOWires,
    UnknownModule(String),
    UnknownWire(String),
    IncorrectWireKind(String),
    MultipleDrivers(String),
    NoDriver(String),
}

impl ParsedModule {
    fn find_wire(&self, name: &String) -> Option<(usize, &Wire)> {
        for (idx, wire) in self.locals.iter().enumerate() {
            if &wire.name == name {
                return Some((idx, &wire));
            }
        }
        None
    }

    fn alloc_wirebus(&self, allocated_wires: &Vec<Vec<usize>>, bus: &WireBus, drive_count: &mut HashMap<&String, Vec<usize>>, modify_count: bool) -> Result<Vec<usize>, LinkError> {
        let mut alloc_bus = Vec::new();
        for part in bus.iter() {
            if let Some((idx, wire)) = self.find_wire(&part.name) {
                let range = if let WireRange::Ranged{from, to} = part.range {
                    from..(to+1)
                } else {
                    0..wire.width
                };
                for i in range {
                    alloc_bus.push(allocated_wires[idx][i]);
                    if modify_count {
                        drive_count.get_mut(&part.name).unwrap()[i] += 1;
                    }
                }
            } else {
                return Err(LinkError::UnknownWire(format!(
                            "In module {}: No local wire with name {}", &self.name, &part.name)));
            }
        }
        Ok(alloc_bus)
    }

    fn link_io(&self, 
        module: &Module<String>, 
        instance: &Instance<String>, 
        io_type: WireKind, 
        allocated_wires: &Vec<Vec<usize>>, 
        child_allocated_wires: &mut Vec<Vec<usize>>,
        drive_count: &mut HashMap<&String, Vec<usize>>) -> Result<(), LinkError> {

        let io = if io_type == WireKind::Input {
            &instance.inputs
        } else {
            &instance.outputs
        };

        for channel in io.iter() {
            let child_wire_name = &channel.module;
            let child_wire_idx = if let Some(idx) = module.locals.iter().position(|c| &c.name == child_wire_name) {
                let child = &module.locals[idx];
                if child.kind != io_type {
                    return Err(LinkError::IncorrectWireKind(format!("")));
                }
                idx
            } else {
                return Err(LinkError::UnknownWire(format!(
                "In module {} in module instantiation {}: No I/O wire with name {}", &self.name, &instance.name, &child_wire_name)));
            };

            child_allocated_wires[child_wire_idx] = self.alloc_wirebus(&allocated_wires, &channel.local, drive_count, io_type == WireKind::Output)?;

            if child_allocated_wires[child_wire_idx].len() != module.locals[child_wire_idx].width {
                return Err(LinkError::MismatchedWireSize(
                        format!("Wire {} of module {} has a wire size of {}, but passed a wire size of {}", 
                            &child_wire_name,  &module.name, module.locals[child_wire_idx].width, child_allocated_wires[child_wire_idx].len())));
            }
        }

        Ok(())
    }

    pub fn link(&self, heritage: &mut HashSet<String>, modules: &HashMap<String, ParsedModule>, net: &mut Net, mut allocated_wires: Vec<Vec<usize>>) -> Result<(), LinkError> {
        if heritage.contains(&self.name) {
            return Err(LinkError::Recursion);
        }

        let mut drive_count = HashMap::new();
        for (idx, wire) in self.locals.iter().enumerate() {
            if let Some(_) = drive_count.insert(&wire.name, vec![0; wire.width]) {
                return Err(LinkError::DuplicateWireName(wire.name.to_owned()));
            }
            if wire.kind == WireKind::Private {
                // allocate space only for private wires, I/O is already allocated by the
                // parent module

                let begin = net.allocate_wire(wire.width);
                allocated_wires[idx] = (begin..begin+wire.width).collect();
            } else {
                assert_eq!(allocated_wires[idx].len(), wire.width);
            }
        }

        if self.name == "nor" {
            net.create_nor(allocated_wires[0][0], allocated_wires[1][0], allocated_wires[2][0]);
            return Ok(());
        }

        for instance in self.instances.iter() {
            if let Some(module) = modules.get(&instance.module) {
                let mut child_allocated_wires = vec![Vec::new(); module.locals.len()];
                self.link_io(module, instance, WireKind::Input, &allocated_wires, &mut child_allocated_wires, &mut drive_count)?;
                self.link_io(module, instance, WireKind::Output, &allocated_wires, &mut child_allocated_wires, &mut drive_count)?;

                // check if all I/O has been assigned
                for (i, wire) in module.locals.iter().enumerate() {
                    if wire.kind == WireKind::Private {
                        assert_eq!(child_allocated_wires[i].len(), 0);
                    } else {
                        if child_allocated_wires[i].len() == 0 {
                            return Err(LinkError::MissingIOWires);
                        } 
                        assert_eq!(child_allocated_wires[i].len(), wire.width);
                    }
                }

                heritage.insert(self.name.to_owned());
                module.link(heritage, modules, net, child_allocated_wires)?;
                heritage.remove(&self.name);
            } else {
                return Err(LinkError::UnknownModule(format!(
                            "In module {}: No module with name {}", &self.name, &instance.module)));
            }
        }

        for (wire_idx, wire) in self.locals.iter().enumerate() {
            let expected_dc: usize = if wire.kind == WireKind::Input {
                0
            } else {
                1
            };

            for (bit_idx, bit) in drive_count.get(&wire.name).unwrap().iter().enumerate() {
                if *bit > expected_dc {
                    return Err(LinkError::MultipleDrivers(format!(
                                "Bit {} in wire {} in module {} is being driven {} times, expected {} times.",
                                bit_idx, &wire.name, &self.name, bit, expected_dc
                    )));
                } else if *bit < expected_dc {
                    continue; // FIXME
                    return Err(LinkError::NoDriver(format!(
                                "Bit {} in wire {} in module {} is not being driven.",
                                bit_idx, &wire.name, &self.name
                    )));
                }
            }
        }

        Ok(())
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

