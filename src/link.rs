use crate::parsed::*;
use crate::netgraph::*;
use crate::net::*;

use std::collections::*;

#[derive(Debug)]
pub enum ErrorKind {
    Recursion,
    MismatchedWireSize,
    DuplicateWireName,
    MissingIOWires,
    UnknownModule,
    UnknownWire,
    IncorrectWireKind,
    MultipleDrivers,
    NoDriver,
}

#[derive(Debug)]
pub struct LinkError {
    pub description: String, 
    pub kind: ErrorKind,
}

impl LinkError {
    fn new(kind: ErrorKind, description: String) -> Self {
        Self { description, kind }
    }
}

pub type LinkResult<T> = Result<T, LinkError>;

pub struct Linker<'a> {
    /// module that is currently being linked
    module: &'a Module,
    /// where the bits of the wire are located in the single 'wire list' in the net
    allocated_wires: &'a mut Vec<Vec<usize>>,
    /// all modules that are being linked, used for finding child modules
    modules: &'a HashMap<String, Module>,
    /// how many times a wire is being edited, on a bit per bit basis
    wire_edits: Vec<Vec<usize>>,
    /// parent modules
    descent: &'a mut HashSet<String>, // TODO maybe replace by a simple vec?
    /// circuit net as output
    net: &'a mut Net,
}

impl<'a> Linker<'a> {
    pub fn new(
        module: &'a Module, 
        allocated_wires: &'a mut Vec<Vec<usize>>, 
        modules: &'a HashMap<String, Module>, 
        descent: &'a mut HashSet<String>, 
        net: &'a mut Net) 
    -> LinkResult<Self> 
    {
        Self::check_duplicate_wires(module)?;

        let mut wire_edits = Vec::with_capacity(module.locals.len());
        for wire in module.locals.iter() {
            // input wires are already edited by the parent module
            let edits = (wire.kind == WireKind::Input) as usize;
            wire_edits.push(vec![edits; wire.width]);
        }

        Ok(Self {
            module, 
            allocated_wires,
            modules,
            wire_edits,
            descent,
            net
        })
    }

    fn check_duplicate_wires(module: &Module) -> LinkResult<()> {
        for (i, wire1) in module.locals.iter().enumerate() {
            for (k, wire2) in module.locals.iter().enumerate() {
                if wire1.name == wire2.name && i != k{
                    return Err(LinkError::new(
                            ErrorKind::DuplicateWireName,
                            format!("Wire '{}' is being defined twice", wire1.name)
                    ));
                }
            }
        }
        Ok(())
    }

    fn alloc_wirebus(&mut self, bus: &'a WireBus, edits_local_wires: bool) -> LinkResult<Vec<usize>> {
        let mut alloc_bus = Vec::new();
        for part in bus.iter() {
            match part {
                WirePart::Local{name, range} => {
                    if let Some((idx, wire)) = self.find_wire(&name) {
                        let range = if let WireRange::Ranged{from, to} = range {
                            *from..(to+1)
                        } else {
                            0..wire.width
                        };
                        for i in range {
                            alloc_bus.push(self.allocated_wires[idx][i]);
                            self.wire_edits[idx][i] += edits_local_wires as usize;
                        }
                    } else {
                        return Err(LinkError::new(
                                ErrorKind::UnknownWire, 
                                format!("In module '{}': No local wire with name '{}'.", self.module.name, &name)
                        ));
                    }

                },
                WirePart::Constant(constant) => {
                    let begin_const = self.net.allocate_wire(constant.len());
                    for (idx, &bit) in constant.iter().enumerate() {
                        alloc_bus.push(begin_const+idx);
                        self.net.set_value(begin_const+idx, bit);
                    }
                }
            }
        }
        Ok(alloc_bus)
    }

    fn find_wire(&self, name: &String) -> Option<(usize, &Wire)> {
        for (idx, wire) in self.module.locals.iter().enumerate() {
            if &wire.name == name {
                return Some((idx, &wire));
            }
        }
        None
    }

    fn link_io(&mut self, module: &'a Module, instance: &'a Instance, io_type: WireKind, child_allocated_wires: &mut Vec<Vec<usize>>) 
        -> Result<(), LinkError> {

            let io = match io_type {
                WireKind::Input => &instance.inputs,
                _ => &instance.outputs,
            };

            for io_wire in io.iter() {
                let child_wire_name = &io_wire.module;
                let child_wire_idx = if let Some(idx) = module.locals.iter().position(|c| &c.name == child_wire_name) {
                    let child = &module.locals[idx];
                    if child.kind != io_type {
                        return Err(LinkError::new(
                                ErrorKind::IncorrectWireKind, 
                                format!(
                                    "Module Instantiation '{}' in '{}': Wire '{}' is not of the correct type.",
                                    instance.name, self.module.name, child_wire_name
                                )
                        ));
                    }
                    idx
                } else {
                    return Err(LinkError::new(
                            ErrorKind::UnknownWire,
                            format!(
                                "In module '{}' in module instantiation '{}': No I/O wire with name '{}'", 
                                self.module.name, &instance.name, &child_wire_name)
                    ));
                };

                child_allocated_wires[child_wire_idx] = self.alloc_wirebus(&io_wire.local, io_type == WireKind::Output)?;

                if child_allocated_wires[child_wire_idx].len() != module.locals[child_wire_idx].width {
                    return Err(LinkError::new(ErrorKind::MismatchedWireSize,
                            format!("Wire {} of module {} has a wire size of {}, but passed a wire size of {}", 
                                &child_wire_name,  &module.name, module.locals[child_wire_idx].width, child_allocated_wires[child_wire_idx].len())));
                }
            }

            Ok(())
        }

    pub fn link(&mut self) -> Result<GraphModule, LinkError> {
        if self.descent.contains(&self.module.name) {
            return Err(LinkError::new(ErrorKind::Recursion, format!("Module {} has a recursive definition", self.module.name)));
        }

        for (idx, wire) in self.module.locals.iter().enumerate() {
            if wire.kind == WireKind::Private {
                // allocate space only for private wires, I/O is already allocated by the
                // parent module

                let begin = self.net.allocate_wire(wire.width);
                self.allocated_wires[idx] = (begin..begin+wire.width).collect();
            } else {
                assert_eq!(self.allocated_wires[idx].len(), wire.width);
            }
        }

        if self.module.name == "Nor" {
            let a = self.allocated_wires[0][0];
            let b = self.allocated_wires[1][0];
            let out = self.allocated_wires[2][0];
            self.net.create_nor(a, b, out);

            return Ok(GraphModule {
                module_name: self.module.name.to_owned(),
                name: String::from("<nor>"),
                instances: Vec::new(),
                locals: vec![
                    GraphWire {
                        name: String::from("a"),
                        values: vec![a],
                    },
                    GraphWire {
                        name: String::from("b"),
                        values: vec![b],
                    },
                    GraphWire {
                        name: String::from("out"),
                        values: vec![out],
                    },
                ],
            });
        }

        let mut instances = Vec::new();

        for instance in self.module.instances.iter() {
            if let Some(module) = self.modules.get(&instance.module) {
                let mut child_allocated_wires = vec![Vec::new(); module.locals.len()];
                self.link_io(module, instance, WireKind::Input, &mut child_allocated_wires)?;
                self.link_io(module, instance, WireKind::Output, &mut child_allocated_wires)?;

                // check if all I/O has been assigned
                for (i, wire) in module.locals.iter().enumerate() {
                    if wire.kind == WireKind::Private {
                        assert_eq!(child_allocated_wires[i].len(), 0);
                    } else {
                        if child_allocated_wires[i].len() == 0 {
                            return Err(LinkError::new(ErrorKind::MissingIOWires, 
                                    format!("Wire {} in Instance {} in Module {} has not been assigned", wire.name, instance.name, self.module.name)));
                        } 
                        assert_eq!(child_allocated_wires[i].len(), wire.width);
                    }
                }

                self.descent.insert(self.module.name.to_owned());
                let mut module_linker = Linker::new(module, &mut child_allocated_wires, self.modules, self.descent, self.net)?;
                let mut graph_instance = module_linker.link()?;
                graph_instance.name = instance.name.to_owned();
                instances.push(graph_instance);
                self.descent.remove(&self.module.name);
            } else {
                return Err(LinkError::new(ErrorKind::UnknownModule,format!(
                            "In module {}: No module with name {}", self.module.name, &instance.module)));
            }
        }

        for (wire_idx, wire) in self.module.locals.iter().enumerate() {
            let expected_dc: usize = if wire.kind == WireKind::Input {
                0
            } else {
                1
            };

            for (bit_idx, bit) in self.wire_edits[wire_idx].iter().enumerate() {
                if *bit > expected_dc {
                    return Err(LinkError::new(ErrorKind::MultipleDrivers,format!(
                                "Bit {} in wire {} in module {} is being driven {} times, expected {} times.",
                                bit_idx, &wire.name, self.module.name, bit, expected_dc
                    )));
                } else if *bit < expected_dc {
                    return Err(LinkError::new(ErrorKind::NoDriver,format!(
                                "Bit {} in wire {} in module {} is not being driven.",
                                bit_idx, &wire.name, self.module.name
                    )));
                }
            }
        }
        Ok(GraphModule{
            module_name: self.module.name.to_owned(),
            name: String::from("<root>"),
            instances,
            locals: {
                let mut locals = Vec::new();
                for (idx, wire) in self.module.locals.iter().enumerate() {
                    let values = self.allocated_wires[idx].clone();
                    locals.push(GraphWire {
                        name: wire.name.to_owned(),
                        values,
                    });
                }
                locals
            },
        })
    }
}

