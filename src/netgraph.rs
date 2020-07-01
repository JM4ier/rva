use std::fmt;

pub struct GraphModule {
    module_name: String,
    name: String,
    locals: Vec<GraphWire>,
    instances: Vec<GraphModule>,
}

pub struct GraphWire {
    name: String,
    values: Vec<usize>,
}

pub enum DisplayError {
    InvalidPath(String),
}

pub struct WireDisplayer {
}

impl WireDisplayer {
    fn display_wire(&self, wire: &[usize]) -> String {
        unimplemented!();
    }
}

impl GraphWire {
    fn display(&self, wd: &WireDisplayer) -> String {
        format!("{}: {}", self.name, wd.display_wire(&self.values))
    }
}

impl GraphModule {
    fn display_locals(&self, wd: &WireDisplayer) -> String {
        let mut lines = String::new();
        for wire in self.locals.iter() {
            lines.push_str(&format!("{}\n", wire.display(wd)));
        }
        lines
    }

    fn display_instances(&self) -> String {
        let mut lines = String::new();
        for inst in self.instances.iter() {
            lines.push_str(&format!("{}::{}\n", inst.module_name, inst.name));
        }
        lines
    }

    pub fn display(&self, wd: &WireDisplayer) -> String {
        format!("{}::{}\nWires:\n{}\nInstances:\n{}\n", self.module_name, self.name, self.display_locals(wd), self.display_instances())
    }

    pub fn display_path(&self, path: &[String], wd: &WireDisplayer) -> Result<String, DisplayError> {
        if path.is_empty() {
            Ok(self.display(wd))
        } else {
            if let Some(i) = self.instances.iter().position(|i| i.name == path[0]) {
                self.instances[i].display_path(&path[1..], wd)
            } else if let Some(i) = self.locals.iter().position(|w| w.name == path[0]) {
                Ok(self.locals[i].display(wd))
            } else {
                Err(DisplayError::InvalidPath(format!("No field with name '{}' in module '{}'", path[0], self.name)))
            }
        }
    }
}

