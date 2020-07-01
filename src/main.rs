mod parsed;
mod parsing;
mod net;

use nom;
use parsed::*;
use parsing::*;

fn main() {
    let str_module = 
"module not (in) -> (out) {
    nor inv(a=in, b=in) -> (out=out);
}";

    println!("{:#?}", module_header(str_module));
    println!("{:#?}", module(str_module));

}
