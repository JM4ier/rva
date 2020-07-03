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

fn bit(i: &str) -> IResult<&str, bool> {
    map(
        alt((char('0'), char('1'))),
        |bit| bit == '1'
    )(i)
}


fn binary_number(i: &str) -> IResult<&str, Vec<bool>> {
    map(
        preceded(opt(tag("0b")), many1(bit)),
        |v| v.into_iter().rev().collect()
    )(i)
}

fn hex_digit(i: &str) -> IResult<&str, Vec<bool>> {
    map (
        take_while_m_n(1, 1, |c: char| c.is_digit(16)), 
        |s| {
            let value = u8::from_str_radix(s, 16).unwrap();
            let mut vec = vec![false; 4];
            for bit in 0..4 {
                vec[bit] = value >> bit > 0;
            }
            vec
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
        binary_number,
        hex_number,
    ))(i)
}

pub fn local_name(i: &str) -> IResult<&str, String> {
    map(
        tuple((
                take_while1(|c: char| c.is_ascii_alphabetic()), 
                take_while(|c: char| c.is_ascii_alphanumeric() || c == '_')
        )),
        |(s1, s2): (&str, &str)| {
            let mut owned = s1.to_string();
            owned.push_str(s2);
            owned
        }
    )(i)
}

pub fn module_name(i: &str) -> IResult<&str, String> {
    map(
        tuple((
                take_while1(|c: char| c.is_ascii_alphabetic() && c.is_ascii_uppercase()), 
                take_while(|c: char| c.is_ascii_alphanumeric() || c == '_')
        )),
        |(s1, s2): (&str, &str)| {
            let mut owned = s1.to_string();
            owned.push_str(s2);
            owned
        }
    )(i)
}

#[test]
fn local_name_test() {
    assert_eq!(local_name("thisisaname"), Ok(("", "thisisaname".to_string())));
}

pub fn number(i: &str) -> IResult<&str, usize> {
    map_res(
        take_while1(|c: char| c.is_digit(10)),
        |i| usize::from_str_radix(i, 10)
    )(i)
}

#[test]
fn number_test() {
    assert_eq!(number("1234"), Ok(("", 1234)));
    assert_eq!(number("9872 other_text 99"), Ok((" other_text 99", 9872)));
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

#[test]
fn range_test() {
    assert_eq!(range("[5:1]"), Ok(("", ((5, 1)))));
}

fn index(i: &str) -> IResult<&str, usize> {
    delimited(
        tag("["),
        number,
        tag("]"),
    )(i)
}

#[test]
fn index_test() {
    assert_eq!(index("[27]"), Ok(("", 27)));
}

fn wirepart(i: &str) -> IResult<&str, WirePart> {
    alt((
            map(
                tuple((local_name, range)),
                |(id, range)| WirePart::ranged(id, range.0, range.1)
            ),
            map(local_name,
                |id| WirePart::total(id),
            ),
            map(
                wire_constant,
                |c| WirePart::constant(c)
            ),
    ))(i)
}

#[test]
fn wirepart_test() {
    assert_eq!(wirepart("asdf "), Ok((" ", WirePart::total("asdf".to_string()))));
    assert_eq!(wirepart("test[1:2] other stuff"), Ok((" other stuff", WirePart::ranged("test".to_string(), 1, 2))));
}

fn comment(i: &str) -> IResult<&str, &str> {
    preceded(
        tag("//"),
        take_until("\n"),
    )(i)
}

pub fn ws(i: &str) -> IResult<&str, &str> {
    terminated(
        take_while(|c: char| c.is_ascii_whitespace()),
        opt(comment),
    )(i)
}

#[test]
fn ws_test() {
    assert_eq!(ws("  \n\t ...\n"), Ok(("...\n", "  \n\t ")));
    assert_eq!(ws(" word "), Ok(("word ", " ")));
}

pub fn list<'a, T, F: Copy + Fn(&str) -> IResult<&str, T>> (parser: F, delimiter: &'a str) -> impl Fn(&'a str) -> IResult<&'a str, Vec<T>> {
    map(
        tuple((
                many0(
                    map(
                        tuple((parser, ws, tag(delimiter), ws)),
                        |(parsed, _, _, _)| parsed
                    )
                ),
                opt(parser),
        )),
        |(list, last)| list.into_iter().chain(last.into_iter()).collect()
    )
}

#[test]
fn list_test() {
    assert_eq!(list(local_name, ",")("a, b, c"), Ok(("", ["a", "b", "c"].iter().map(|s| s.to_string()).collect())));
    assert_eq!(list(local_name, ",")(")"), Ok((")", vec![])));
    assert_eq!(list(local_name, ",")("a, b), c"), Ok(("), c", vec![String::from("a"), String::from("b")])));
}

fn wirebus(i: &str) -> IResult<&str, WireBus> {
    alt((
            delimited(
                tag("{"),
                list(wirepart, ","),
                tag("}")
            ),
            map(
                wirepart,
                |wp| vec![wp],
            )
    ))(i)
}

#[test]
fn wirebus_test() {
    assert_eq!(wirebus("{a, b[3:4], c[0]} "), Ok((" ", vec![
                WirePart::total("a".to_string()), 
                WirePart::ranged("b".to_string(), 3, 4), 
                WirePart::ranged("c".to_string(), 0, 0)])));
}

fn wire(i: &str) -> IResult<&str, Wire> {
    alt((
            map(
                tuple((
                        local_name,
                        index,
                )),
                |(name, width)| Wire {
                    name,
                    width,
                    kind: WireKind::Private,
                }
            ),
            map(
                local_name,
                |name| Wire { 
                    name, 
                    width: 1, 
                    kind: WireKind::Private },
            )
    ))(i)
}

#[test]
fn wire_test() {
    assert_eq!(wire("peter[5]"), Ok(("", Wire { name: "peter".to_string(), width: 5, kind: WireKind::Private })));
    assert_eq!(wire("hans "), Ok((" ", Wire { name: "hans".to_string(), width: 1, kind: WireKind::Private })));
}

fn local_wire(i: &str) -> IResult<&str, Vec<Wire>> {
    map(
        tuple((
                tag("wire"),
                ws,
                list(wire, ","),
                tag(";"),
        )),
        |(_, _, w, _)| w
    )(i)
}

#[test]
fn local_wire_test() {
    assert_eq!(local_wire("wire rudolf; ..."), Ok((" ...", Wire {
        name: "rudolf".to_string(),
        width: 1,
        kind: WireKind::Private,
    })));
    assert_eq!(local_wire("wire stefan[278];"), Ok(("", Wire {
        name: "stefan".to_string(),
        width: 278,
        kind: WireKind::Private,
    })));
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
                        local_name,
                        ws,
                        tag("="),
                        ws,
                        wirebus,
                )),
                |(name, _, _, _, bus)| Connection { local: bus, module: name }
            ),
            map(
                local_name,
                |name| Connection { local: vec![WirePart::total(name.to_string())], module: name },
            )
    ))(i)
}

#[test]
fn assignment_test() {
    assert_eq!(assignment("a=b"), Ok(("", Connection::<String> {
        module: "a".to_string(),
        local: vec![WirePart::total("b".to_string())]
    })));
    assert_eq!(assignment("a = {c[2], d[1:4], f}"), Ok(("", Connection::<String> {
        module: "a".to_string(),
        local: vec![
            WirePart::ranged("c".to_string(), 2, 2),
            WirePart::ranged("d".to_string(), 1, 4),
            WirePart::total("f".to_string())
        ]
    })));
}

fn instance(i: &str) -> IResult<&str, Instance> {
    map(
        tuple((
                ws,
                local_name,
                ws,
                local_name,
                ws,
                delimited(
                    tag("("),
                    list(assignment, ","),
                    tag(")"),
                ),
                ws,
                tag("->"),
                ws,
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

#[test]
fn instance_test() {
    assert_eq!(instance("nor inv(a=in, b=in) -> (out=out);"), Ok(("", 
                Instance{
                    module: "nor".to_string(),
                    name: "inv".to_string(),
                    inputs: vec![
                        Connection::<String> {
                            module: "a".to_string(),
                            local: vec![WirePart::total("in".to_string())]
                        },
                        Connection::<String> {
                            module: "b".to_string(),
                            local: vec![WirePart::total("in".to_string())]
                        },
                    ],
                    outputs: vec![Connection::<String> {
                        module: "out".to_string(),
                        local: vec![WirePart::total("out".to_string())]
                    }],
                }
    )));
}

pub fn module_header(i: &str) -> IResult<&str, (String, Vec<Wire>, Vec<Wire>)> {
    map(
        tuple((
                ws,
                tag("module"),
                ws,
                module_name,
                ws,
                delimited(
                    tag("("),
                    list(input_wire, ","),
                    tag(")"),
                ),
                ws,
                tag("->"),
                ws,
                delimited(
                    tag("("),
                    list(output_wire, ","),
                    tag(")"),
                ),
                ws
        )),
        |(_, _, _, name, _, inputs, _, _, _, outputs, _)| (name, inputs, outputs),
        )(i)
}

pub enum BodyOption {
    LocalWire(Vec<Wire>),
    Instance(Instance),
}

pub fn body_option (i: &str) -> IResult<&str, BodyOption> {
    alt((
            map(local_wire, |w| BodyOption::LocalWire(w)),
            map(instance, |i| BodyOption::Instance(i)),
    ))(i)
}

pub fn module_body(i: &str) -> IResult<&str, Vec<BodyOption>> {
    delimited(
        tag("{"),
        delimited(
            ws,
            many0(delimited(ws, body_option, ws)),
            ws,
        ),
        tuple((tag("}"), ws))
    )(i)
}

pub fn module(i: &str) -> IResult<&str, Module> {
    map(
        tuple((module_header, ws, module_body)),
        |((name, mut inputs, mut outputs), _,  body)| {
            let mut locals = Vec::new();
            let mut instances = Vec::new();

            locals.append(&mut inputs);
            locals.append(&mut outputs);

            for line in body {
                match line {
                    BodyOption::LocalWire(mut w) => locals.append(&mut w),
                    BodyOption::Instance(i) => instances.push(i),
                }
            }

            Module { name, locals, instances }
        }
    )(i)
}

pub fn modules(i: &str) -> IResult<&str, Vec<Module>> {
    many0(module)(i)
}
