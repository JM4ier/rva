use std::fmt;

#[derive(Debug)]
pub struct GraphModule {
    pub module_name: String,
    pub name: String,
    pub locals: Vec<GraphWire>,
    pub instances: Vec<GraphModule>,
}

#[derive(Debug)]
pub struct GraphWire {
    pub name: String,
    pub values: Vec<usize>,
}

#[derive(Debug)]
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

