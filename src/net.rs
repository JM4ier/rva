use std::collections::*;

#[derive(Debug)]
pub struct Net {
    wires: Vec<bool>,
    gates: Vec<NorGate>,
}

#[derive(Debug)]
pub struct NorGate {
    in1: usize,
    in2: usize,
    out: usize,
}

#[derive(Debug)]
pub struct Simulation {
    net: Net,
    /// which gates are dependent on which wires
    dependencies: Vec<Vec<usize>>,
    /// gates that are potentially in an unstable state
    dirty: Vec<bool>,
    /// queue of dirty gates that need processing
    process_queue: VecDeque<usize>,
}

impl Net {
    /// allocates space in the netlist for a wire and returns its 'address'
    pub fn allocate_wire(&mut self, width: usize) -> usize {
        let begin = self.wires.len();
        self.wires.append(&mut vec![false; width]);
        begin
    }

    /// creates a new nor gate with specified I/O
    pub fn create_nor(&mut self, in1: usize, in2: usize, out: usize) {
        assert!(in1 < self.wires.len());
        assert!(in2 < self.wires.len());
        assert!(out < self.wires.len());

        self.gates.push(NorGate { in1, in2, out });
    }

    pub fn new() -> Self {
        Self {
            wires: Vec::new(),
            gates: Vec::new(),
        }
    }
}

impl Simulation {
    pub fn new(net: Net) -> Self {
        let dirty = vec![true; net.gates.len()];
        let mut process_queue = VecDeque::with_capacity(net.gates.len());

        let mut dependencies = vec![Vec::new(); net.wires.len()];
        for (idx, gate) in net.gates.iter().enumerate() {
            dependencies[gate.in1].push(idx);
            dependencies[gate.in2].push(idx);

            process_queue.push_back(idx);
        }

        for dep in dependencies.iter_mut() {
            dep.dedup();
        }

        Self {
            net,
            dependencies,
            dirty,
            process_queue,
        }
    }

    #[inline]
    pub fn update(&mut self) {
        if let Some(gate) = self.process_queue.pop_front() {
            self.dirty[gate] = false;
            let gate =  &self.net.gates[gate];

            let in1 = self.net.wires[gate.in1];
            let in2 = self.net.wires[gate.in2];
            let orig = self.net.wires[gate.out];

            let out = !(in1 || in2);

            if orig != out {
                self.net.wires[gate.out] = out;

                for &d in self.dependencies[gate.out].iter() {
                    if !self.dirty[d] {
                        self.dirty[d] = true;
                        self.process_queue.push_back(d);
                    }
                }
            }
        }
    }

    #[inline]
    pub fn is_stable(&self) -> bool {
        self.process_queue.is_empty()
    }
}

