use super::*;

use crate::parsed::*;

#[test]
fn hex_digit_test() {
    assert_eq!(hex_digit("F"), Ok(("", vec![true; 4])));
    assert_eq!(hex_digit("1"), Ok(("", vec![true, false, false, false])));
    assert_eq!(hex_digit("2"), Ok(("", vec![false, true, false, false])));
    assert_eq!(hex_digit("7"), Ok(("", vec![true, true, true, false])));
}

#[test]
fn hex_number_test() {
    assert_eq!(hex_number("0x42"), Ok(("", vec![false, true, false, false, false, false, true, false])));
    match hex_number("0xC0FFEE") {
        Err(e) => assert!(false, "Couldn't parse 0xC0FFEE"),
        Ok((rest, num)) => {
            assert_eq!(rest, "");
            assert_eq!(num.len(), 24);
        },
    }
}

#[test]
fn field_name_test() {
    assert_eq!(field_name("thisisaname"), Ok(("", "thisisaname".to_string())));
}

#[test]
fn number_test() {
    assert_eq!(number("1234"), Ok(("", 1234)));
    assert_eq!(number("9872 other_text 99"), Ok((" other_text 99", 9872)));
}

#[test]
fn range_test() {
    assert_eq!(range("[5:1]"), Ok(("", ((5, 1)))));
}

#[test]
fn index_test() {
    assert_eq!(index("[27]"), Ok(("", 27)));
}

#[test]
fn wirepart_test() {
    assert_eq!(
        wirepart("asdf "), 
        Ok((
                " ", 
                WirePart::total("asdf")
        ))
    );
    assert_eq!(
        wirepart("test[1:2] other stuff"), 
        Ok((
                " other stuff", 
                WirePart::ranged("test", 1, 2)
        ))
    );
}

#[test]
fn whitespace_test() {
    assert_eq!(whitespace("  \n\t ...\n"), Ok(("...\n", "  \n\t ")));
    assert_eq!(whitespace(" word "), Ok(("word ", " ")));
}

#[test]
fn list_test() {
    assert_eq!(
        list(field_name, ",")("a, b, c"), 
        Ok(("", ["a", "b", "c"].iter().map(|s| s.to_string()).collect()))
    );
    assert_eq!(
        list(field_name, ",")(")"), Ok((")", 
            vec![]))
    );
    assert_eq!(
        list(field_name, ",")("a, b), c"), 
        Ok(("), c", vec![String::from("a"), String::from("b")]))
    );
}

#[test]
fn wirebus_test() {
    assert_eq!(wirebus("{a, b[3:4], c[0]} "), Ok((" ", vec![
                WirePart::total("a"),
                WirePart::ranged("b", 3, 4),
                WirePart::ranged("c", 0, 0)])));
}

#[test]
fn wirebus_repeat_test() {
    assert_eq!(
        wirebus("5 * {0}"),
        Ok(("", vec![WirePart::Constant(vec![false]); 5]))
    );
    assert_eq!(
        wirebus("3 * {0b01}"),
        Ok(("", vec![WirePart::Constant(vec![true, false]); 3]))
    );
}

#[test]
fn wire_test() {
    assert_eq!(
        wire("peter[5]"), 
        Ok(("", Wire { 
            name: "peter".to_string(), 
            width: 5, 
            kind: WireKind::Private 
        }))
    );
    assert_eq!(
        wire("hans "), 
        Ok((" ", Wire { 
            name: "hans".to_string(), 
            width: 1, 
            kind: WireKind::Private 
        }))
    );
}

#[test]
fn local_wire_test() {
    assert_eq!(local_wire("wire rudolf; ..."), Ok((" ...", vec![Wire {
        name: "rudolf".to_string(),
        width: 1,
        kind: WireKind::Private,
    }])));
    assert_eq!(local_wire("wire stefan[278];"), Ok(("", vec![Wire {
        name: "stefan".to_string(),
        width: 278,
        kind: WireKind::Private,
    }])));
}

#[test]
fn io_binding_test() {
    assert_eq!(io_binding("a=b"), Ok(("", Connection {
        module: "a".to_string(),
        local: vec![WirePart::total("b")]
    })));
    assert_eq!(io_binding("a = {c[2], d[1:4], f}"), Ok(("", Connection {
        module: "a".to_string(),
        local: vec![
            WirePart::ranged("c", 2, 2),
            WirePart::ranged("d", 1, 4),
            WirePart::total("f")
        ]
    })));
}

#[test]
fn instance_test() {
    assert_eq!(instance("Nor inv(a=in, b=in) -> (out=out);"), Ok(("", 
                Instance{
                    module: "Nor".to_string(),
                    name: "inv".to_string(),
                    inputs: vec![
                        Connection {
                            module: "a".to_string(),
                            local: vec![WirePart::total("in")]
                        },
                        Connection {
                            module: "b".to_string(),
                            local: vec![WirePart::total("in")]
                        },
                    ],
                    outputs: vec![Connection {
                        module: "out".to_string(),
                        local: vec![WirePart::total("out")]
                    }],
                }
    )));
}

#[test]
fn operation_parentheses_test() {
    assert_eq!(
        operation("(a)"),
        Ok((
                "",
                Operation::Wire(
                    vec![WirePart::total("a")]
                )
        ))
    );
}

#[test]
fn binary_operation_test() {
    assert_eq!(
        operation("a & b"),
        Ok((
                "",
                Operation::And(
                    Box::new(Operation::Wire(vec![WirePart::total("a")])),
                    Box::new(Operation::Wire(vec![WirePart::total("b")])),
                )
        ))
    );
}

#[test]
fn binary_paren_operation_test() {
    assert_eq!(
        operation("(a | b) & (c ^ d)"),
        Ok((
                "",
                Operation::And(
                    Box::new(
                        Operation::Or(
                            Box::new(Operation::Wire(vec![WirePart::total("a")])),
                            Box::new(Operation::Wire(vec![WirePart::total("b")])),
                        )
                    ),
                    Box::new(
                        Operation::Xor(
                            Box::new(Operation::Wire(vec![WirePart::total("c")])),
                            Box::new(Operation::Wire(vec![WirePart::total("d")])),
                        )
                    )
                )
        ))
    );
}

#[test]
fn unary_operation_test() {
    assert_eq!(
        operation("&(a | b)"),
        Ok((
                "",
                Operation::AndReduce(
                    Box::new(
                        Operation::Or(
                            Box::new(Operation::Wire(vec![WirePart::total("a")])),
                            Box::new(Operation::Wire(vec![WirePart::total("b")])),
                        )
                    ),
                )
        ))
    );
}

#[test]
fn wire_assignment_test() {
    assert_eq!(
        wire_assignment("wire[5:10] = (!in1[0:5] | in2) & in3;"),
        Ok(("",
                WireAssignment {
                    wire: vec![WirePart::ranged("wire", 5, 10)],
                    operation:
                        Operation::And(
                            Box::new(Operation::Or(
                                    Box::new(Operation::Not(
                                            Box::new(Operation::Wire(vec![WirePart::ranged("in1", 0, 5)])),
                                    )),
                                    Box::new(Operation::Wire(vec![WirePart::total("in2")]))
                            )),
                            Box::new(Operation::Wire(vec![WirePart::total("in3")])),
                        ),
                }
        ))
    );
}

#[test]
#[should_panic]
fn unparsed_module_causes_error_test() {
    // causes error because module names need to be uppercase
    modules("module mod() -> () {}").unwrap();
}

