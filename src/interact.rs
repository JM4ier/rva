use nom::*;

use crate::net::*;
use crate::netgraph::*;
use crate::parsing::*;

unsafe fn path<'a>(ptr: *const u8, len: u64) -> IResult<&'a str, Vec<String>> {
    let slice = std::slice::from_raw_parts(ptr, len as _);
    let i = std::str::from_utf8(slice).unwrap();
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
    let path = match path(path_ptr, path_len) {
        Ok((_, path)) => path,
        Err(e) => {
            eprintln!("Error parsing path: {:?}", e);
            return 0;
        },
    };

    match graph.wire_addr(&path) {
        Ok(addr) => {
            let mut vec = addr.iter().map(|&a| sim.get_value(a)).collect::<Vec<bool>>();
            vec.shrink_to_fit();
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

    let path = match path(path_ptr, path_len) {
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
pub unsafe extern "C" fn drop_bools(vec: *mut bool, len: usize) {
    let vec = Vec::from_raw_parts(vec, len, len);
    std::mem::drop(vec);
}

#[no_mangle]
pub unsafe extern "C" fn drop_chars(vec: *mut u8, len: usize) {
    let vec = Vec::from_raw_parts(vec, len, len);
    std::mem::drop(vec);
}

#[no_mangle]
pub unsafe extern "C" fn get_description(
    sim: &Simulation, 
    graph: &GraphModule, 
    path_ptr: *const u8, path_len: u64,
    description_ptr: &mut *const u8)
-> usize 
{
    let mut result = (|| {
        let location = match path(path_ptr, path_len) {
            Ok((_, path)) => path,
            Err(e) => return format!("Error: {:?}", e),
        };

        match graph.display_path(String::new(), &location, sim) {
            Ok(s) => s,
            Err(e) => format!("Error: {:?}", e),
        }
    })();

    result.shrink_to_fit();
    *description_ptr = result.as_bytes().as_ptr();
    let len = result.len();
    std::mem::forget(result);
    len
}

#[no_mangle]
pub unsafe extern "C" fn get_width(graph: &GraphModule, path_ptr: *const u8, path_len: u64) -> u64 {
    let path = match path(path_ptr, path_len) {
        Ok((_, path)) => path,
        Err(e) => {
            format!("Error: {:?}", e);
            return 0;
        },
    };
    graph.wire_width(&path).unwrap_or(0)
}

