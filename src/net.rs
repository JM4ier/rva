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

    pub fn set_value(&mut self, idx: usize, val: bool) {
        self.wires[idx] = val;
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
    fn enqueue_dependencies(&mut self, wire: usize) {
        for &gate in self.dependencies[wire].iter() {
            if !self.dirty[gate] {
                self.dirty[gate] = true;
                self.process_queue.push_back(gate);
            }
        }
    }

    #[inline]
    fn dequeue(&mut self) -> Option<usize> {
        let front = self.process_queue.pop_front();
        if let Some(gate) = front {
            self.dirty[gate] = false;
        }
        front
    }

    #[inline]
    pub fn update(&mut self) {
        if let Some(gate) = self.dequeue() {
            let gate =  &self.net.gates[gate];

            let in1 = self.net.wires[gate.in1];
            let in2 = self.net.wires[gate.in2];
            let orig = self.net.wires[gate.out];

            let out = !(in1 || in2);

            if orig != out {
                self.net.wires[gate.out] = out;
                let wire = gate.out;
                self.enqueue_dependencies(wire);
            }
        }
    }

    #[inline]
    pub fn is_stable(&self) -> bool {
        self.process_queue.is_empty()
    }

    #[inline]
    pub fn set_value(&mut self, addr: usize, value: bool) {
        self.net.wires[addr] = value;
        self.enqueue_dependencies(addr);
    }
}

impl crate::netgraph::WireDisplayer for Simulation {
    fn display_wire(&self, wire: &[usize]) -> String {
        let mut string = String::from("0b");
        for &i in wire.iter().rev() {
            string += &format!("{}", self.net.wires[i] as u8);
        }
        string
    }
}

