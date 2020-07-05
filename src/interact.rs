use std::ffi::CString;

use nom::*;

use crate::net::*;
use crate::netgraph::*;
use crate::parsing::*;

fn path(i: &str) -> IResult<&str, Vec<String>> {
    list(field_name, ".")(i)
}

#[no_mangle]
pub unsafe extern "C" fn simulate(sim: &mut Simulation, mut count: u64) -> bool {
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
pub unsafe extern "C" fn get_value(
    sim: &Simulation, 
    graph: &GraphModule, 
    path_ptr: *const u8, path_len: u64, 
    buffer: *mut *mut bool) 
-> usize 
{
    let location = std::slice::from_raw_parts(path_ptr, path_len as _);
    let location = std::str::from_utf8(location).unwrap();

    let path = match path(&location) {
        Ok((_, path)) => path,
        Err(e) => {
            eprintln!("Error parsing path: {:?}", e);
            return 0;
        },
    };

    match graph.wire_addr(&path) {
        Ok(addr) => {
            let mut vec = addr.iter().map(|&a| sim.get_value(a)).collect::<Vec<bool>>();
            *buffer = vec.as_mut_ptr();
            let len = vec.len();
            std::mem::forget(vec);
            len
        },
        Err(e) => {
            eprintln!("Error: {:?}", e);
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn set_value(
    sim: &mut Simulation, 
    graph: &GraphModule, 
    path_ptr: *const u8, path_len: u64, 
    values: *const bool, values_len: u64) 
{
    let values = std::slice::from_raw_parts(values, values_len as _);
    let location = std::slice::from_raw_parts(path_ptr, path_len as _);
    let location = std::str::from_utf8(location).unwrap();

    let path = match path(location) {
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

#[no_mangle]
pub unsafe extern "C" fn drop_buffer(vec: *mut bool, len: usize) {
    let vec = Vec::from_raw_parts(vec, len, len);
    std::mem::drop(vec);
}

