use nom::{
    *,
    bytes::complete::*,
    combinator::*,
    sequence::*,
    branch::*,
    multi::*,
};

use crate::parsed::*;

fn identifier(i: &str) -> IResult<&str, String> {
    map(
        take_while1(|c: char| c.is_ascii_alphabetic()),
        str::to_string,
    )(i)
}

fn number(i: &str) -> IResult<&str, usize> {
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
            map(identifier,
                |id| WirePart::total(id),
            ),
            map(
                tuple((identifier, range)),
                |(id, range)| WirePart::ranged(id, range.0, range.1)
            ))
    )(i)
}

fn ws(i: &str) -> IResult<&str, &str> {
    take_while(
        |c: char| c.is_ascii_whitespace()
    )(i)
}

fn delim(i: &str) -> IResult<&str, ()> {
    map(
        take_while(
            |c: char| c == ',' || c.is_ascii_whitespace()
        ),
        |_|()
    )(i)
}

fn list<'a, T, F: Copy + Fn(&str) -> IResult<&str, T>> (parser: F) -> impl Fn(&'a str) -> IResult<&'a str, Vec<T>> {
    map(
        tuple((
                many0(
                    map(
                        tuple((parser, delim)),
                        |(parsed, _)| parsed
                    )
                ),
                opt(parser),
        )),
        |(list, last)| list.into_iter().chain(last.into_iter()).collect()
    )
}

fn wirebus(i: &str) -> IResult<&str, WireBus> {
    alt((
            delimited(
                tag("{"),
                list(wirepart),
                tag("}")
            ),
            map(
                wirepart,
                |wp| vec![wp],
            )
    ))(i)
}

fn local_wire(i: &str) -> IResult<&str, Wire> {
    map(
        tuple((
                identifier,
                index,
        )),
        |(name, width)| Wire {
            name,
            width,
            kind: WireKind::Private,
        }
    )(i)
}

fn input_wire(i: &str) -> IResult<&str, Wire> {
    unimplemented!();
}

fn output_wire(i: &str) -> IResult<&str, Wire> {
    unimplemented!();
}

fn assignment(i: &str) -> IResult<&str, (String, WireBus)> {
    map(
        tuple((
                identifier,
                ws,
                tag("="),
                ws,
                wirebus,
        )),
        |(name, _, _, _, bus)| {
            unimplemented!();
        }
    )(i)
}

fn instance(i: &str) -> IResult<&str, Instance<String>> {
    map(
        tuple((
                identifier,
                ws,
                identifier,
                ws,
                delimited(
                    tag("("),
                    list(assignment),
                    tag(")"),
                ),
        )),
        |(module, _, name, _, assignments)| {
            unimplemented!();
        }
    )(i)
}

