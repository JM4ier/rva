use nom::{
    *,
    bytes::complete::*,
    combinator::*,
    character::complete::*,
    sequence::*,
    branch::*,
    multi::*,
};

use crate::parsed::*;

#[cfg(test)]
mod tests;

fn bit(i: &str) -> IResult<&str, bool> {
    map(
        alt((char('0'), char('1'))),
        |bit| bit == '1'
    )(i)
}


fn binary_number(i: &str) -> IResult<&str, Vec<bool>> {
    map(
        preceded(opt(tag("0b")), many1(bit)),
        // MSB is typically written left(low index), so reverse order to match little endian
        |v| v.into_iter().rev().collect()
    )(i)
}

fn hex_digit(i: &str) -> IResult<&str, Vec<bool>> {
    map (
        take_while_m_n(1, 1, |c: char| c.is_digit(16)), 
        |s| {
            let digit = u8::from_str_radix(s, 16).unwrap();
            let mut bits = vec![false; 4];
            for idx in 0..4 {
                bits[idx] = (digit >> idx)&1 > 0;
            }
            bits
        }
    )(i)
}

fn hex_number(i: &str) -> IResult<&str, Vec<bool>> {
    map(
        preceded(tag("0x"), many1(hex_digit)),
        |v| v.into_iter().rev().flat_map(|v| v.into_iter()).collect()
    )(i)
}

pub fn wire_constant(i: &str) -> IResult<&str, Vec<bool>> {
    alt((
            hex_number,
            binary_number,
    ))(i)
}

pub fn field_name(i: &str) -> IResult<&str, String> {
    map(
        tuple((
                take_while1(|c: char| c.is_ascii_alphabetic()), 
                take_while(|c: char| c.is_ascii_alphanumeric() || c == '_')
        )),
        |(s1, s2): (&str, &str)| s1.to_owned() + s2
    )(i)
}

pub fn module_name(i: &str) -> IResult<&str, String> {
    map(
        tuple((
                take_while1(|c: char| c.is_ascii_alphabetic() && c.is_ascii_uppercase()), 
                take_while(|c: char| c.is_ascii_alphanumeric())
        )),
        |(s1, s2): (&str, &str)| s1.to_owned() + s2
    )(i)
}

pub fn number(i: &str) -> IResult<&str, usize> {
    map_res(
        take_while1(|c: char| c.is_digit(10)),
        |i| usize::from_str_radix(i, 10)
    )(i)
}

fn range(i: &str) -> IResult<&str, (usize, usize)> {
    delimited(
        tag("["),
        alt((
                map(
                    tuple((number, tag(":"), number)),
                    |(from, _, to)| (from, to),
                ),
                map(number, |num| (num, num))
        )),
        tag("]")
    )(i)
}

fn index(i: &str) -> IResult<&str, usize> {
    delimited(
        tag("["),
        number,
        tag("]"),
    )(i)
}

fn wirepart(i: &str) -> IResult<&str, WirePart> {
    alt((
            map(
                tuple((field_name, range)),
                |(id, range)| WirePart::ranged(id, range.0, range.1)
            ),
            map(field_name,
                |id| WirePart::total(id),
            ),
            map(
                wire_constant,
                |c| WirePart::constant(c)
            ),
    ))(i)
}

fn comment(i: &str) -> IResult<&str, &str> {
    preceded(
        tag("//"),
        take_until("\n"),
    )(i)
}

pub fn whitespace(i: &str) -> IResult<&str, &str> {
    terminated(
        take_while(|c: char| c.is_ascii_whitespace()),
        opt(comment),
    )(i)
}

pub fn list<'a, T, F: Copy + Fn(&str) -> IResult<&str, T>> 
(parser: F, delimiter: &'a str) -> impl Fn(&'a str) -> IResult<&'a str, Vec<T>> 
{
    map(
        tuple((
                many0(
                    map(
                        tuple((parser, whitespace, tag(delimiter), whitespace)),
                        |(parsed, _, _, _)| parsed
                    )
                ),
                opt(parser),
        )),
        |(list, last)| list.into_iter().chain(last.into_iter()).collect()
    )
}

fn repeat(i: &str) -> IResult<&str, usize> {
    terminated(
        number,
        tuple((whitespace, tag("*"), whitespace)),
    )(i)
}

fn repeating_wirepart(i: &str) -> IResult<&str, Vec<WirePart>> {
    map(
        tuple((
                opt(repeat),
                wirepart,
        )),
        |(reps, part)| vec![part.clone(); reps.unwrap_or(1)]
    )(i)
}

fn wirebus(i: &str) -> IResult<&str, WireBus> {
    map(
        tuple((
                opt(repeat),
                alt((
                        delimited(
                            tag("{"),
                            list(repeating_wirepart, ","),
                            tag("}")
                        ),
                        map(
                            repeating_wirepart,
                            |wp| vec![wp],
                        )
                ))
        )),
        |(reps, bus)| {
            let bus: Vec<_> = bus.into_iter().flat_map(|w| w.into_iter()).collect();
            let elems = reps.unwrap_or(1) * bus.len();
            bus.into_iter().cycle().take(elems).collect()
        }
    )(i)
}

fn wire(i: &str) -> IResult<&str, Wire> {
    alt((
            map(
                tuple((
                        field_name,
                        index,
                )),
                |(name, width)| Wire {
                    name,
                    width,
                    kind: WireKind::Private,
                }
            ),
            map(
                field_name,
                |name| Wire { 
                    name, 
                    width: 1, 
                    kind: WireKind::Private },
            )
    ))(i)
}

fn local_wire(i: &str) -> IResult<&str, Vec<Wire>> {
    map(
        tuple((
                tag("wire"),
                whitespace,
                list(wire, ","),
                tag(";"),
        )),
        |(_, _, w, _)| w
    )(i)
}

fn input_wire(i: &str) -> IResult<&str, Wire> {
    map(wire, |w| Wire { kind: WireKind::Input, ..w})(i)
}

fn output_wire(i: &str) -> IResult<&str, Wire> {
    map(wire, |w| Wire { kind: WireKind::Output, ..w})(i)
}

fn assignment(i: &str) -> IResult<&str, Connection> {
    alt((
            map(
                tuple((
                        field_name,
                        whitespace,
                        tag("="),
                        whitespace,
                        wirebus,
                )),
                |(name, _, _, _, bus)| Connection { 
                    local: bus, 
                    module: name 
                }
            ),
            map(
                field_name,
                |name| Connection { 
                    local: vec![WirePart::total(&name)],
                    module: name
                },
            )
                ))(i)
}

fn instance(i: &str) -> IResult<&str, Instance> {
    map(
        tuple((
                whitespace,
                module_name,
                whitespace,
                field_name,
                whitespace,
                delimited(
                    tag("("),
                    list(assignment, ","),
                    tag(")"),
                ),
                whitespace,
                tag("->"),
                whitespace,
                delimited(
                    tag("("),
                    list(assignment, ","),
                    tag(")"),
                ),
                tag(";"),
        )),
        |(_, module, _, name, _, inputs, _, _, _, outputs, _)| { 
            Instance { module, name, inputs, outputs } 
        }
    )(i)
}

fn unary_operation<'a, F>(op_tag: &'static str, fun: F) -> impl Fn(&'a str) -> IResult<&'a str, Operation>
where F: Copy + Fn(Box<Operation>) -> Operation {
    move |i: &'a str| {
        map(
            preceded(
                tuple((whitespace, tag(op_tag), whitespace)),
                operation_literal
            ),
            |op| fun(Box::new(op))
        )(i)
    }
}

fn binary_operation<'a, F> (op_tag: &'static str, fun: F) -> impl Fn(&'a str) -> IResult<&'a str, Operation> 
where F: Copy + Fn(Box<Operation>, Box<Operation>) -> Operation {
    move |i: &'a str| {
        map(
            tuple((
                    whitespace,
                    operation_literal,
                    whitespace,
                    tag(op_tag),
                    whitespace,
                    operation,
                    whitespace
            )),
            |tup| fun(Box::new(tup.1), Box::new(tup.5))
        )(i)
    }
}

fn operation_literal(i: &str) -> IResult<&str, Operation> {
    alt((
            map(wirebus, Operation::Wire),
            delimited(
                tuple((whitespace, tag("("), whitespace)),
                operation,
                tuple((whitespace, tag(")"), whitespace)),
            ),
            unary_operation("!", Operation::Not),
            unary_operation("&", Operation::AndReduce),
            unary_operation("|", Operation::OrReduce),
            unary_operation("^", Operation::XorReduce),
    ))(i)
}

fn operation(i: &str) -> IResult<&str, Operation> {
    alt((
            binary_operation("&", Operation::And),
            binary_operation("|", Operation::Or),
            binary_operation("^", Operation::Xor),
            operation_literal,
    ))(i)
}

fn module_header(i: &str) -> IResult<&str, (String, Vec<Wire>, Vec<Wire>)> {
    map(
        tuple((
                whitespace,
                tag("module"),
                whitespace,
                module_name,
                whitespace,
                delimited(
                    tag("("),
                    list(input_wire, ","),
                    tag(")"),
                ),
                whitespace,
                tag("->"),
                whitespace,
                delimited(
                    tag("("),
                    list(output_wire, ","),
                    tag(")"),
                ),
                whitespace
        )),
        |(_, _, _, name, _, inputs, _, _, _, outputs, _)| (name, inputs, outputs)
    )(i)
}

enum BodyPart {
    LocalWire(Vec<Wire>),
    Instance(Instance),
}

fn body_part (i: &str) -> IResult<&str, BodyPart> {
    alt((
            map(local_wire, |w| BodyPart::LocalWire(w)),
            map(instance, |i| BodyPart::Instance(i)),
    ))(i)
}

fn module_body(i: &str) -> IResult<&str, Vec<BodyPart>> {
    delimited(
        tag("{"),
        delimited(
            whitespace,
            many0(delimited(whitespace, body_part, whitespace)),
            whitespace,
        ),
        tuple((tag("}"), whitespace))
    )(i)
}

fn module(i: &str) -> IResult<&str, Module> {
    map(
        tuple((module_header, whitespace, module_body)),
        |((name, mut inputs, mut outputs), _,  body)| {
            let mut locals = Vec::new();
            let mut instances = Vec::new();

            locals.append(&mut inputs);
            locals.append(&mut outputs);

            for line in body {
                match line {
                    BodyPart::LocalWire(mut w) => locals.append(&mut w),
                    BodyPart::Instance(i) => instances.push(i),
                }
            }

            Module { name, locals, instances }
        }
    )(i)
}

pub fn modules(i: &str) -> IResult<&str, Vec<Module>> {
    let (mut rest, mut modules) = many0(module)(i)?;
    if rest.len() > 0 {
        // should return an error, as there is an unparsed rest that is 
        // apparently not a valid module
        module(rest)?;
    }
    Ok((rest, modules))
}

