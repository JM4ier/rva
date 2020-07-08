use crate::parsed::*;

pub struct Resolver<'a> {
    module: &'a mut Module,
    counter: u64,
}

impl<'a> Resolver<'a> {
    pub fn new (module: &'a mut Module) -> Self {
        Self {
            module,
            counter: 0,
        }
    }

    pub fn resolve_assignments(&mut self, assignments: Vec<WireAssignment>) -> Result<(), ()> {
        for assignment in assignments.into_iter() {
            self.resolve_assignment(assignment)?;
        }
        Ok(())
    }

    fn generate_name(&mut self) -> String {
        let name = format!("gen_{}", self.counter);
        self.counter += 1;
        name
    }

    fn resolve_assignment(&mut self, assignment: WireAssignment) -> Result<(), ()> {
        let bus = assignment.bus;
        let operation = assignment.operation;
        self.resolve_operation(operation, Some(bus))?;
        Ok(())
    }

    fn unary_operation(&mut self, input: &WireBus, output: &WireBus, gate_type: &str) -> Result<(), ()> {
        let width = self.bus_width(input)?;
        for i in 0..width {
            let ini = self.index_bus(input, i)?;
            let outi = self.index_bus(output, i)?;
            let name = self.generate_name();
            let gate = Instance {
                module: gate_type.to_string(),
                name: name.to_string(),
                inputs: vec![Connection {
                    local: ini,
                    module: String::from("in"),
                }],
                outputs: vec![Connection {
                    local: outi,
                    module: String::from("out"),
                }],
            };
            self.module.instances.push(gate);
        }
        Ok(())
    }

    fn binary_operation(&mut self, in1: &WireBus, in2: &WireBus, out: &WireBus, gate_type: &str) -> Result<(), ()> {
        let width = self.bus_width(in1)?;
        for i in 0..width {
            let in1i = self.index_bus(in1, i)?;
            let in2i = self.index_bus(in2, i)?;
            let outi = self.index_bus(out, i)?;
            let name = self.generate_name();
            let gate = Instance {
                module: gate_type.to_string(),
                name: name.to_string(),
                inputs: vec![
                    Connection {
                        local: in1i,
                        module: String::from("a"),
                    },
                    Connection {
                        local: in2i,
                        module: String::from("b"),
                    },
                ],
                outputs: vec![Connection {
                    local: outi,
                    module: String::from("out"),
                }],
            };
            self.module.instances.push(gate);
        }
        Ok(())
    }

    fn reduce_operation(&mut self, input: WireBus, out: &WireBus, gate_type: &str) -> Result<(), ()> {
        let width = self.bus_width(&input)?;
        assert!(width > 0);

        if width == 1 {
            // simply connect input to output
            self.unary_operation(&input, out, "Buffer")?;
        } else {
            let (part1, part2) = self.slice_bus(input)?;

            let parts = vec![part1, part2];
            let mut parts_result = vec![];

            for part in parts.into_iter() {
                // only insert a gate if there are more than 2 inputs
                if self.bus_width(&part)? == 1 {
                    parts_result.push(part);
                } else {
                    let wire = self.create_bus(1);
                    self.reduce_operation(part, &wire, gate_type)?;
                    parts_result.push(wire);
                }
            }

            self.binary_operation(&parts_result[0], &parts_result[1], out, gate_type)?;
        }

        Ok(())
    }

    /// slices a bus in half
    fn slice_bus(&self, bus: WireBus) -> Result<(WireBus, WireBus), ()> {
        let width = self.bus_width(&bus)?;
        assert!(width >= 2);
        let mid = width / 2;
        let mut part1 = Vec::new();
        let mut part2 = Vec::new();
        for i in 0..width {
            let mut partial = self.index_bus(&bus, i)?;
            if i < mid {
                part1.append(&mut partial);
            } else {
                part2.append(&mut partial);
            }
        }
        Ok((part1, part2))
    }

    fn resolve_operation(&mut self, op: Operation, output: Option<WireBus>) -> Result<WireBus, ()> {
        let width = op.width(self.module)?;

        if let Operation::Wire(bus) = op {
            match output {
                Some(o) => { 
                    self.unary_operation(&bus, &o, "Buffer")?; 
                    return Ok(o);
                },
                None => return Ok(bus),
            }
        }

        let output = match output {
            Some(o) => o,
            None => self.create_bus(width),
        };

        match op {
            Operation::Wire(_) => unreachable!(),

            // TODO make this less repetitive?
            Operation::Not(op) => {
                let input = self.resolve_operation(*op, None)?;
                self.unary_operation(&input, &output, "Not")?;
            },
            Operation::And(op1, op2) => {
                let in1 = self.resolve_operation(*op1, None)?;
                let in2 = self.resolve_operation(*op2, None)?;
                self.binary_operation(&in1, &in2, &output, "And")?;
            },
            Operation::Or(op1, op2) => {
                let in1 = self.resolve_operation(*op1, None)?;
                let in2 = self.resolve_operation(*op2, None)?;
                self.binary_operation(&in1, &in2, &output, "Or")?;
            },
            Operation::Xor(op1, op2) => {
                let in1 = self.resolve_operation(*op1, None)?;
                let in2 = self.resolve_operation(*op2, None)?;
                self.binary_operation(&in1, &in2, &output, "Xor")?;
            },
            Operation::AndReduce(op) => {
                let input = self.resolve_operation(*op, None)?;
                self.reduce_operation(input, &output, "And")?;
            },
            Operation::OrReduce(op) => {
                let input = self.resolve_operation(*op, None)?;
                self.reduce_operation(input, &output, "Or")?;
            },
            Operation::XorReduce(op) => {
                let input = self.resolve_operation(*op, None)?;
                self.reduce_operation(input, &output, "Xor")?;
            },
        }
        Ok(output)
    }

    fn create_bus(&mut self, width: usize) -> WireBus {
        let wire = self.create_wire(width);
        vec![WirePart::ranged(wire, 0, width-1)]
    }

    /// creates a wire in the module with a generated name and returns the name
    fn create_wire(&mut self, width: usize) -> String {
        let name = self.generate_name();
        let wire = Wire {
            name: name.clone(),
            kind: WireKind::Private,
            width,
        };
        self.module.locals.push(wire);
        name
    }

    fn index_bus(&self, bus: &WireBus, mut index: usize) -> Result<WireBus, ()> {
        for part in bus.iter() {
            let width = part.width(self.module)?;
            if width > index {
                let indexed_part = match part {
                    WirePart::Constant(c) => {
                        WirePart::constant(vec![c[index]])
                    },
                    WirePart::Local{name, range: WireRange::Total} => {
                        WirePart::ranged(name, index, index)
                    },
                    WirePart::Local{name, range: WireRange::Ranged{from, to: _}} => {
                        WirePart::ranged(name, from+index, from+index)
                    },
                };
                return Ok(vec![indexed_part]);
            }
            index -= width;
        }
        Err(())
    }

    fn bus_width(&self, bus: &WireBus) -> Result<usize, ()> {
        bus.iter().map(|w| w.width(self.module)).sum()
    }
}

