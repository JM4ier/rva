use nom::{
    *,
    bytes::complete::*,
    combinator::*,
    sequence::*,
    branch::*,
    multi::*,
};

use std::io::{self, prelude::*};

use crate::net::*;
use crate::netgraph::*;
use crate::parsing::*;

enum Command {
    Print(Vec<String>),
    Edit(Vec<String>, Vec<bool>),
    Simulate(Option<usize>),
    Terminate,
}

fn path(i: &str) -> IResult<&str, Vec<String>> {
    list(identifier, ".")(i)
}

fn command(i: &str) -> IResult<&str, Command>  {
    alt((
            map(
                preceded(tuple((tag("print"), ws)), path),
                |path| Command::Print(path)
            ),
            map(
                tuple((tag("assign"), ws, path, ws, tag("="), ws, wire_constant)),
                |(_, _, path, _, _, _, constant)| Command::Edit(path, constant)
            ),
            map(
                alt((tag("terminate"), tag("stop"), tag("quit"))),
                |_| Command::Terminate,
            ),
            map(
                tuple((tag("run"), ws, opt(number))),
                |(_, _, no)| Command::Simulate(no)
            )
    ))(i)
}

pub fn run_interactive(netgraph: &GraphModule, sim: &mut Simulation) -> io::Result<()> {
    let mut input = String::new();
    let stdin = io::stdin();

    loop {
        print!("Type something: ");
        io::stdout().flush();

        input.clear();
        stdin.read_line(&mut input)?;
        let cmd = command(&input);

        if let Ok((_, cmd)) = cmd {
            match cmd {
                Command::Terminate => break,
                Command::Print(path) => {
                    let display = &netgraph.display_path(&path, sim);
                    match display {
                        Ok(s) => println!("\n{}", s),
                        Err(e) => eprintln!("\n\nError: {:?}\n\n", e),
                    }
                },
                Command::Edit(path, values) => {
                    let addr = &netgraph.wire_addr(&path);
                    match addr {
                        Ok(addr) => {
                            let it = addr.iter().zip(values.iter());
                            for (i, (&addr, &val)) in it.enumerate() {
                                sim.set_value(addr, val);
                            }
                            if values.len() < addr.len() {
                                println!("Warning, passed value has {} bits, but wire needs {} bits.", values.len(), addr.len());
                            }
                        },
                        Err(e) => eprintln!("\n\nError: {:?}\n\n", e),
                    }
                },
                Command::Simulate(mut count) => {
                    while sim.is_stable() {
                        sim.update();
                        if let Some(0) = count {
                            break;
                        }
                        count = count.map(|c| c-1);
                    }
                },
            }
        } else {
            println!("Error parsing input");
        }
    }
    Ok(())
}

