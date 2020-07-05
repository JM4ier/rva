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

impl GraphModule {
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

