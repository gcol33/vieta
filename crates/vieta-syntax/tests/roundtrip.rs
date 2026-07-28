//! The syntax-layer acceptance properties from D37.

use vieta_syntax::{
    BinaryOp, Cst, ElementRef, NodeKind, NodeRef, Origin, SourceText, Syntax, TokenKind, lower,
    parse,
};

/// A structural fingerprint that ignores where nodes happen to sit in the
/// arena, which is what "the same tree" means when comparing two parses.
fn shape(cst: &Cst) -> String {
    let mut out = String::new();
    write_node(cst.root(), &mut out);
    out
}

fn write_node(node: NodeRef<'_>, out: &mut String) {
    out.push('(');
    out.push_str(&format!("{:?}", node.kind()));
    for child in node.children() {
        out.push(' ');
        match child {
            ElementRef::Node(node) => write_node(node, out),
            ElementRef::Token(token) if token.is_synthetic() => {
                out.push_str(&format!("missing:{:?}", token.kind()));
            }
            ElementRef::Token(token) => {
                out.push_str(&format!("{:?}={:?}", token.kind(), token.text()));
            }
        }
    }
    out.push(')');
}

/// The invariant the round trip rests on: the leaves that came from the source
/// tile it exactly, in order.
fn leaves_tile(cst: &Cst) -> bool {
    let mut offset = 0;
    for leaf in cst.leaves() {
        if leaf.is_synthetic() {
            continue;
        }
        if leaf.span().start != offset {
            return false;
        }
        offset = leaf.span().end;
    }
    offset == cst.source().len()
}

fn check(source: &str) {
    let tree = parse(SourceText::from(source));
    assert_eq!(tree.print(), source, "print(parse(s)) != s for {source:?}");
    assert!(leaves_tile(&tree), "leaves do not tile {source:?}");

    let again = parse(SourceText::from(tree.print()));
    assert_eq!(shape(&again), shape(&tree), "reparsing changed the tree for {source:?}");
}

const CORPUS: &[&str] = &[
    "",
    " ",
    "x",
    "x + 2",
    "2 + x",
    "  x  +  2  ",
    "\u{feff}x + 1",
    "x\r\ny",
    "\tx\t",
    "f(x, y)",
    "f(g(h(x)))",
    "f(x)(y)",
    "((x))",
    "-x^2",
    "a^b^c",
    "a - b - c",
    "let a = 1 in a + 1",
    "fn(x) => x * x",
    "let f = fn(x) => x ^ 2 in f(3)",
    "term { x + 2 }",
    "term{fn(x) => x + y}",
    "term { x } + 1",
    "x // comment",
    "x /* why */ + y",
    "/* unclosed",
    "1_000 + 1.50",
    "1.",
    "\u{03b1} + \u{03b2}",
    // Malformed, and lossless all the same.
    "f(x",
    "f(x,)",
    "f(,)",
    "(",
    ")",
    "x +",
    "+ x",
    "let",
    "fn",
    "let a = in",
    "fn(x) x",
    "term",
    "term {",
    "term x",
    "{ x }",
    "@#$",
    "x @ y",
    "x y",
    "1 2 3",
];

#[test]
fn printing_a_tree_reproduces_its_source() {
    for source in CORPUS {
        check(source);
    }
}

#[test]
fn deeply_nested_input_still_round_trips() {
    let deep = format!("{}x{}", "(".repeat(200), ")".repeat(200));
    check(&deep);
    let chained = "1".to_owned() + &" + 1".repeat(300);
    check(&chained);
}

/// A deterministic stream, so a failure is reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as usize
    }
}

#[test]
fn generated_token_soup_round_trips() {
    const FRAGMENTS: &[&str] = &[
        "x", "y", "1", "1.5", "+", "-", "*", "/", "^", "(", ")", "{", "}", ",", "=", "=>", "fn",
        "let", "in", "term", " ", "\t", "\r\n", "// c\n", "/* c */", "@", "\u{03b1}", "\u{feff}",
        "_",
    ];
    let mut rng = Rng(0x51ee_d001);
    for _ in 0..2_000 {
        let count = rng.next() % 12;
        let mut source = String::new();
        for _ in 0..count {
            source.push_str(FRAGMENTS[rng.next() % FRAGMENTS.len()]);
        }
        check(&source);
    }
}

#[test]
fn recovery_never_fabricates_bytes() {
    let tree = parse(SourceText::from("f(x"));
    assert_eq!(tree.print(), "f(x");
    assert_eq!(tree.errors().len(), 1);

    let inserted: Vec<_> = tree.leaves().filter(|leaf| leaf.is_synthetic()).collect();
    assert_eq!(inserted.len(), 1);
    assert_eq!(inserted[0].kind(), TokenKind::RParen);
    assert_eq!(inserted[0].text(), "");
    assert_eq!(inserted[0].span().start, 3, "it names where the paren was wanted");
}

#[test]
fn trivia_are_leaves_in_source_order() {
    let tree = parse(SourceText::from("x + /* why */ y"));
    let kinds: Vec<_> = tree.leaves().map(|leaf| leaf.kind()).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Ident,
            TokenKind::Whitespace,
            TokenKind::Plus,
            TokenKind::Whitespace,
            TokenKind::BlockComment,
            TokenKind::Whitespace,
            TokenKind::Ident,
        ]
    );
}

#[test]
fn invalid_utf8_is_refused_before_lexing() {
    let result = SourceText::new(&[b'x', b' ', b'+', 0x80]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().offset, 3);
}

#[test]
fn a_byte_order_mark_survives_the_round_trip() {
    let source = "\u{feff}x + 1";
    let tree = parse(SourceText::from(source));
    assert_eq!(tree.print(), source);
    assert_eq!(tree.leaves().next().expect("a leaf").kind(), TokenKind::Bom);
    assert!(tree.errors().is_empty());
}

#[test]
fn line_endings_and_tabs_are_not_normalized() {
    for source in ["x\r\n+ y", "x\n+ y", "x\t+\ty"] {
        assert_eq!(parse(SourceText::from(source)).print(), source);
    }
}

#[test]
fn number_spelling_survives_to_surface_syntax() {
    for spelling in ["1_000", "1.50", "007"] {
        let tree = parse(SourceText::from(spelling));
        match lower(&tree) {
            Syntax::Number { text, .. } => assert_eq!(&*text, spelling),
            other => panic!("{spelling:?} lowered to {other:?}"),
        }
    }
}

// Grouping is a fact about the source, so the concrete trees differ and the
// programs do not.
#[test]
fn parentheses_survive_below_syntax_and_not_above_it() {
    let grouped = parse(SourceText::from("(x + y)"));
    let plain = parse(SourceText::from("x + y"));
    assert_ne!(shape(&grouped), shape(&plain));
    assert!(lower(&grouped).same_shape(&lower(&plain)));

    let redundant = parse(SourceText::from("((x) + ((y)))"));
    assert!(lower(&redundant).same_shape(&lower(&plain)));
}

#[test]
fn comments_and_whitespace_do_not_reach_surface_syntax() {
    let spaced = parse(SourceText::from("x   /* why */ +\n\ty"));
    let plain = parse(SourceText::from("x+y"));
    assert!(lower(&spaced).same_shape(&lower(&plain)));
}

#[test]
fn precedence_and_associativity_reach_surface_syntax() {
    let sum = lower(&parse(SourceText::from("a + b * c")));
    let grouped = lower(&parse(SourceText::from("a + (b * c)")));
    assert!(sum.same_shape(&grouped));

    let power = lower(&parse(SourceText::from("a ^ b ^ c")));
    let right = lower(&parse(SourceText::from("a ^ (b ^ c)")));
    assert!(power.same_shape(&right), "^ associates to the right");

    let subtraction = lower(&parse(SourceText::from("a - b - c")));
    let left = lower(&parse(SourceText::from("(a - b) - c")));
    assert!(subtraction.same_shape(&left), "- associates to the left");

    let negated = lower(&parse(SourceText::from("-x^2")));
    let over_power = lower(&parse(SourceText::from("-(x^2)")));
    assert!(negated.same_shape(&over_power), "unary minus is looser than ^");

    let scaled = lower(&parse(SourceText::from("-x*y")));
    let over_product = lower(&parse(SourceText::from("(-x)*y")));
    assert!(scaled.same_shape(&over_product), "unary minus is tighter than *");
}

#[test]
fn binding_forms_reach_surface_syntax_with_their_names() {
    match lower(&parse(SourceText::from("let a = 1 in a + 1"))) {
        Syntax::Let { name, value, body, .. } => {
            assert_eq!(&*name, "a");
            assert!(matches!(*value, Syntax::Number { .. }));
            assert!(matches!(*body, Syntax::Binary { op: BinaryOp::Add, .. }));
        }
        other => panic!("let lowered to {other:?}"),
    }

    match lower(&parse(SourceText::from("fn(x) => x * x"))) {
        Syntax::Lambda { parameter, body, .. } => {
            assert_eq!(&*parameter, "x");
            assert!(matches!(*body, Syntax::Binary { op: BinaryOp::Multiply, .. }));
        }
        other => panic!("fn lowered to {other:?}"),
    }
}

// A quotation body parses like anything else, so what the form changes is
// elaboration and not the grammar.
#[test]
fn quotation_reaches_surface_syntax_around_an_ordinary_body() {
    match lower(&parse(SourceText::from("term { fn(x) => x + y }"))) {
        Syntax::Quote { body, .. } => match *body {
            Syntax::Lambda { parameter, .. } => assert_eq!(&*parameter, "x"),
            other => panic!("quotation body lowered to {other:?}"),
        },
        other => panic!("term lowered to {other:?}"),
    }

    let spaced = lower(&parse(SourceText::from("term {  x + y  }")));
    let tight = lower(&parse(SourceText::from("term{x+y}")));
    assert!(spaced.same_shape(&tight));
}

#[test]
fn calls_nest_and_carry_their_arguments() {
    match lower(&parse(SourceText::from("f(x, g(y))"))) {
        Syntax::Call { callee, arguments, .. } => {
            assert!(matches!(*callee, Syntax::Name { .. }));
            assert_eq!(arguments.len(), 2);
            assert!(matches!(arguments[1], Syntax::Call { .. }));
        }
        other => panic!("call lowered to {other:?}"),
    }
}

#[test]
fn every_surface_node_carries_where_it_came_from() {
    let tree = parse(SourceText::from("x + 2"));
    let syntax = lower(&tree);
    assert!(matches!(syntax.origin(), Origin::Source(_)));
    let span = syntax.origin().span();
    assert_eq!(tree.source().slice(span), Some("x + 2"));

    let broken = lower(&parse(SourceText::from("f(x")));
    assert!(
        matches!(broken.origin(), Origin::Recovered(_)),
        "a node the parser had to complete says so"
    );
}

#[test]
fn the_root_of_empty_source_is_an_error_rather_than_a_panic() {
    let tree = parse(SourceText::from(""));
    assert_eq!(tree.print(), "");
    assert!(matches!(lower(&tree), Syntax::Error { .. }));
    assert_eq!(tree.root().kind(), NodeKind::Root);
}
