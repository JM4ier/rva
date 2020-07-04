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
pub enum Error {
    InvalidPath(String),
}

pub trait WireDisplayer {
    fn display_wire(&self, wire: &[usize]) -> String;
}

impl GraphWire {
    fn display<WD: WireDisplayer>(&self, wd: &WD) -> String {
        format!("    {}: {}", self.name, wd.display_wire(&self.values))
    }
}

impl GraphModule {
    fn display_locals<WD: WireDisplayer>(&self, wd: &WD) -> String {
        let mut lines = String::new();
        for wire in self.locals.iter() {
            lines.push_str(&format!("{}\n", wire.display(wd)));
        }
        lines
    }

    fn display_instances(&self) -> String {
        let mut lines = String::new();
        for inst in self.instances.iter() {
            lines.push_str(&format!("    {}::{}\n", inst.module_name, inst.name));
        }
        lines
    }

    pub fn display<WD: WireDisplayer>(&self, wd: &WD) -> String {
        format!("{}::{}\n  Wires:\n{}  Instances:\n{}\n", self.module_name, self.name, self.display_locals(wd), self.display_instances())
    }

    pub fn display_path<WD: WireDisplayer>(&self, path: &[String], wd: &WD) -> Result<String, Error> {
        if path.is_empty() {
            Ok(self.display(wd))
        } else {
            if let Some(i) = self.instances.iter().position(|i| i.name == path[0]) {
                self.instances[i].display_path(&path[1..], wd)
            } else if let Some(i) = self.locals.iter().position(|w| w.name == path[0]) {
                Ok(self.locals[i].display(wd))
            } else {
                Err(Error::InvalidPath(format!("No field with name '{}' in module '{}'", path[0], self.name)))
            }
        }
    }

    pub fn wire_addr(&self, path: &[String]) -> Result<Vec<usize>, Error> {
        if path.is_empty() {
            Err(Error::InvalidPath(String::from("This path refers to a module")))
        } else {
            if let Some(i) = self.instances.iter().position(|i| i.name == path[0]) {
                self.instances[i].wire_addr(&path[1..])
            } else if let Some(i) = self.locals.iter().position(|w| w.name == path[0]) {
                Ok(self.locals[i].values.clone())
            } else {
                Err(Error::InvalidPath(format!("No field with name '{}' in module '{}'", path[0], self.name)))
            }
        }
    }
}

