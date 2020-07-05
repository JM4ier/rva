use std::ffi::CString;

use nom::*;

use crate::net::*;
use crate::netgraph::*;
use crate::parsing::*;

fn path(i: &str) -> IResult<&str, Vec<String>> {
    list(field_name, ".")(i)
}

#[no_mangle]
pub unsafe extern "C" fn simulate(sim: *mut Simulation, mut count: u32) -> bool {
    let sim = sim.as_mut().unwrap(); // if the caller gives us an invalid pointer, it's their fault
    let bounded = count > 0;

    while !sim.is_stable() {
        sim.update();
        if bounded {
            count -= 1;
            if count == 0 {
                break;
            }
        }
    }

    sim.is_stable()
}

#[no_mangle]
pub unsafe extern "C" fn get_value(sim: *const Simulation, graph: *const GraphModule, location: CString) -> *const bool {
    let sim = sim.as_ref().unwrap();
    let graph = graph.as_ref().unwrap();

    let path = match path(&location.into_string().unwrap()) {
        Ok((_, path)) => path,
        Err(e) => {
            eprintln!("Error parsing path: {:?}", e);
            return vec![].as_ptr();
        },
    };

    match graph.wire_addr(&path) {
        Ok(addr) => {
            addr.iter().map(|&a| sim.get_value(a)).collect::<Vec<bool>>().as_ptr()
        },
        Err(e) => {
            eprintln!("Error: {:?}", e);
            vec![].as_ptr()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn set_value(sim: *mut Simulation, graph: *const GraphModule, location: CString, values: *const bool, values_len: usize) {
    let sim = sim.as_mut().unwrap();
    let graph = graph.as_ref().unwrap();
    let values = std::slice::from_raw_parts(values, values_len);

    let path = match path(&location.into_string().unwrap()) {
        Ok((_, path)) => path,
        Err(e) => {
            eprintln!("Error parsing path: {:?}", e);
            return;
        },
    };

    let addr = &graph.wire_addr(&path);
    match addr {
        Ok(addr) => {
            for (&addr, &val) in addr.iter().zip(values.iter()) {
                sim.set_value(addr, val);
            }
            if values.len() < addr.len() {
                eprintln!("Warning, passed value has {} bits, but wire needs {} bits.", values.len(), addr.len());
            }
        },
        Err(e) => eprintln!("Error: {:?}", e),
    }
}

