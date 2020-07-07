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
    pub kind: ErrorKind,
    pub description: String, 
}

impl LinkError {
    fn new<T>(kind: ErrorKind, description: String) -> LinkResult<T> {
        Err(Self { description, kind })
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
    descent: &'a mut Vec<String>,
    /// circuit net as output
    net: &'a mut Net,
}

impl<'a> Linker<'a> {
    pub fn new(
        module: &'a Module, 
        allocated_wires: &'a mut Vec<Vec<usize>>, 
        modules: &'a HashMap<String, Module>, 
        descent: &'a mut Vec<String>, 
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
                    return LinkError::new(
                        ErrorKind::DuplicateWireName,
                        format!(
                            "Wire '{}' is being defined multiple times.", 
                            wire1.name
                        )
                    );
                }
            }
        }
        Ok(())
    }

    fn alloc_wirebus(&mut self, bus: &'a WireBus, io_type: WireKind) -> LinkResult<Vec<usize>> {
        let mut alloc_bus = Vec::new();
        for part in bus.iter() {
            match part {
                WirePart::Local{name, range} => {
                    if let Some((idx, wire)) = self.find_wire(&name) {
                        let range = if let WireRange::Ranged{from, to} = range {
                            if from > to || *to >= wire.width {
                                return LinkError::new(
                                    ErrorKind::MismatchedWireSize,
                                    format!(
                                        "[{}:{}] is not a valid subset of Wire '{}[{}]' in Module '{}'.",
                                        from, to, wire.name, wire.width, self.module.name
                                    )
                                );
                            }
                            *from..(*to+1)
                        } else {
                            0..wire.width
                        };
                        for i in range {
                            alloc_bus.push(self.allocated_wires[idx][i]);
                            self.wire_edits[idx][i] += (io_type == WireKind::Output) as usize;
                        }
                    } else {
                        return LinkError::new(
                            ErrorKind::UnknownWire, 
                            format!(
                                "In module '{}': No local wire with name '{}'.", 
                                self.module.name, &name
                            )
                        );
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

    fn link_instance_io(
        &mut self, module: &'a Module, 
        instance: &'a Instance, 
        allocated_wires: &mut Vec<Vec<usize>>, 
        io_type: WireKind)
        -> LinkResult<()> 
    {
        let io = match io_type {
            WireKind::Input => &instance.inputs,
            _ => &instance.outputs,
        };

        for io_wire in io.iter() {
            let wire_name = &io_wire.module;
            let wire_idx = 
                match module.locals.iter().position(|c| c.name == *wire_name) {
                    Some(idx) => idx,
                    None => return LinkError::new(
                        ErrorKind::UnknownWire,
                        format!(
                            "In module '{}' in module instantiation '{}': No I/O wire with name '{}'.", 
                            self.module.name, &instance.name, &wire_name
                        )
                    ),
                };
            let child_wire = &module.locals[wire_idx];

            if child_wire.kind != io_type {
                return LinkError::new(
                    ErrorKind::IncorrectWireKind, 
                    format!(
                        "Module Instantiation '{}' in '{}': Wire '{}' is not of the correct type.",
                        instance.name, self.module.name, wire_name
                    )
                );
            }

            allocated_wires[wire_idx] = self.alloc_wirebus(&io_wire.local, io_type)?;

            if allocated_wires[wire_idx].len() != module.locals[wire_idx].width {
                return LinkError::new(
                    ErrorKind::MismatchedWireSize,
                    format!(
                        "Wire '{}' of module '{}' has a wire size of '{}', but passed a wire size of {}.", 
                        &wire_name,  &module.name, module.locals[wire_idx].width, allocated_wires[wire_idx].len()
                    )
                );
            }
        }

        Ok(())
    }

    pub fn link(&mut self) -> LinkResult<GraphModule> {
        if self.descent.contains(&self.module.name) {
            return LinkError::new(
                ErrorKind::Recursion, 
                format!(
                    "Module '{}' has a recursive definition.", 
                    self.module.name
                )
            );
        }

        // base case, everything gets broken down to nor gates
        if self.module.name == "Nor" {
            let a = self.allocated_wires[0][0];
            let b = self.allocated_wires[1][0];
            let out = self.allocated_wires[2][0];
            self.net.create_nor(a, b, out);

            let graph_wire = |name, idx| GraphWire {
                name: String::from(name),
                values: vec![idx],
            };

            return Ok(
                GraphModule {
                    module_name: self.module.name.clone(),
                    name: String::from("<nor>"),
                    instances: Vec::new(),
                    locals: vec![
                        graph_wire("a", a),
                        graph_wire("b", b),
                        graph_wire("out", out),
                    ],
                }
            );
        }

        for (idx, wire) in self.module.locals.iter().enumerate() {
            if wire.kind == WireKind::Private {
                // allocate space only for private wires, 
                // I/O is already allocated by the parent module

                let begin = self.net.allocate_wire(wire.width);
                self.allocated_wires[idx] = (begin..begin+wire.width).collect();
            } else {
                assert_eq!(self.allocated_wires[idx].len(), wire.width);
            }
        }

        let mut graph_instances = Vec::new();

        for instance in self.module.instances.iter() {
            let module = match self.modules.get(&instance.module) {
                Some(m) => m,
                None => return LinkError::new(
                    ErrorKind::UnknownModule,
                    format!(
                        "In module '{}': No module with name '{}'.",
                        self.module.name, &instance.module
                    )
                ),
            };

            let mut allocated_wires = vec![Vec::new(); module.locals.len()];

            use WireKind::*;
            self.link_instance_io(module, instance, &mut allocated_wires, Input)?;
            self.link_instance_io(module, instance, &mut allocated_wires, Output)?;

            // check if all I/O has been assigned
            for (i, wire) in module.locals.iter().enumerate() {
                if wire.kind == WireKind::Private {
                    assert_eq!(allocated_wires[i].len(), 0);
                } else {
                    if allocated_wires[i].len() == 0 {
                        return LinkError::new(
                            ErrorKind::MissingIOWires, 
                            format!(
                                "Wire '{}' in Instance '{}' in Module '{}' has not been assigned.", 
                                wire.name, instance.name, self.module.name
                            )
                        );
                    } 
                    assert_eq!(allocated_wires[i].len(), wire.width);
                }
            }

            self.descent.push(self.module.name.clone());
            {
                let mut module_linker = Linker::new(
                    module, &mut allocated_wires, self.modules, self.descent, self.net
                )?;

                let mut graph_instance = module_linker.link()?;
                graph_instance.name = instance.name.to_owned();
                graph_instances.push(graph_instance);
            }
            self.descent.pop();
        }

        for (wire_idx, wire) in self.module.locals.iter().enumerate() {
            for (bit_idx, &bit) in self.wire_edits[wire_idx].iter().enumerate() {
                if bit > 1 {
                    return LinkError::new(
                        ErrorKind::MultipleDrivers,
                        format!(
                            "wire '{}[{}]' in module '{}' is being driven {} times, expected {} times.",
                            &wire.name, bit_idx, self.module.name, bit, 1
                        )
                    );
                } else if bit < 1 {
                    return LinkError::new(
                        ErrorKind::NoDriver,
                        format!(
                            "wire '{}[{}]' in module '{}' is not being driven.",
                            &wire.name, bit_idx, self.module.name
                        )
                    );
                }
            }
        }

        Ok(
            GraphModule{
                module_name: self.module.name.clone(),
                name: String::from("<root>"),
                instances: graph_instances,
                locals: 
                    self.module.locals
                    .iter()
                    .enumerate()
                    .map(|(idx, wire)| 
                        GraphWire {
                            name: wire.name.clone(),
                            values: self.allocated_wires[idx].clone(),
                        }
                    )
                    .collect()
            }
        )
    }
}

