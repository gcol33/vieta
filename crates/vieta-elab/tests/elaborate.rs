//! The acceptance properties of D38's first elaboration slice.

use vieta_elab::{Elaboration, Literal, Resolved, elaborate, free_binders};
use vieta_store::{CancelToken, Cancelled, Store};
use vieta_syntax::{Origin, SourceText, lower, parse};

fn resolve<'s>(store: &'s Store, source: &str) -> Elaboration<'s> {
    let tree = parse(SourceText::from(source));
    elaborate(store, &lower(&tree)).expect("nothing was cancelled")
}

fn term<'s>(store: &'s Store, source: &str) -> vieta_store::ExprId<'s> {
    match resolve(store, source).into_resolved() {
        Resolved::Quote { term, .. } => term,
        other => panic!("{source:?} elaborated to {other:?}"),
    }
}

#[test]
fn a_lambda_claims_its_parameter_and_leaves_the_rest_global() {
    let store = Store::new();
    let elaborated = resolve(&store, "fn(x) => x + y");

    let Resolved::Lambda { binder, name, body, .. } = elaborated.resolved() else {
        panic!("a lambda elaborates to a lambda");
    };
    assert_eq!(&**name, "x", "the source name survives as a hint");

    let Resolved::Call { callee, arguments, .. } = &**body else {
        panic!("infix elaborates to a call");
    };
    assert!(matches!(&**callee, Resolved::Global { name, .. } if &**name == "+"));
    assert!(matches!(arguments[0], Resolved::Local { binder: b, .. } if b == *binder));
    assert!(matches!(&arguments[1], Resolved::Global { name, .. } if &**name == "y"));
    assert!(elaborated.diagnostics().is_empty());
}

#[test]
fn the_innermost_binder_of_a_name_is_the_one_that_claims_it() {
    let store = Store::new();
    let elaborated = resolve(&store, "fn(x) => fn(x) => x");

    let Resolved::Lambda { binder: outer, body, .. } = elaborated.resolved() else {
        panic!("a lambda elaborates to a lambda");
    };
    let Resolved::Lambda { binder: inner, body: innermost, captures, .. } = &**body else {
        panic!("the body is the inner lambda");
    };
    assert_ne!(outer, inner);
    assert!(matches!(**innermost, Resolved::Local { binder, .. } if binder == *inner));
    assert!(captures.is_empty(), "the inner lambda reaches nothing outside itself");
}

#[test]
fn a_closure_captures_what_its_body_reaches_from_outside() {
    let store = Store::new();
    let elaborated = resolve(&store, "fn(x) => fn(y) => x");

    let Resolved::Lambda { binder: outer, body, captures: outer_captures, .. } =
        elaborated.resolved()
    else {
        panic!("a lambda elaborates to a lambda");
    };
    let Resolved::Lambda { captures: inner_captures, .. } = &**body else {
        panic!("the body is the inner lambda");
    };

    assert_eq!(inner_captures.len(), 1);
    assert_eq!(inner_captures[0].binder, *outer);
    assert_eq!(&*inner_captures[0].name, "x");
    assert!(outer_captures.is_empty(), "the outer lambda binds what the inner one copies");
}

#[test]
fn a_capture_travels_through_every_closure_between_binder_and_use() {
    let store = Store::new();
    let elaborated = resolve(&store, "fn(x) => fn(y) => fn(z) => x");

    let Resolved::Lambda { binder: outer, body: middle, captures: at_outer, .. } =
        elaborated.resolved()
    else {
        panic!("a lambda elaborates to a lambda");
    };
    let Resolved::Lambda { body: inner, captures: at_middle, .. } = &**middle else {
        panic!("the middle lambda");
    };
    let Resolved::Lambda { captures: at_inner, .. } = &**inner else {
        panic!("the inner lambda");
    };

    assert_eq!(at_inner.iter().map(|c| c.binder).collect::<Vec<_>>(), vec![*outer]);
    assert_eq!(at_middle.iter().map(|c| c.binder).collect::<Vec<_>>(), vec![*outer]);
    assert!(at_outer.is_empty());
}

#[test]
fn a_let_binds_its_body_and_not_its_value() {
    let store = Store::new();
    let elaborated = resolve(&store, "let x = x in x");

    let Resolved::Let { binder, value, body, name, .. } = elaborated.resolved() else {
        panic!("a let elaborates to a let");
    };
    assert_eq!(&**name, "x");
    assert!(matches!(&**value, Resolved::Global { .. }), "the value is outside its own scope");
    assert!(matches!(**body, Resolved::Local { binder: b, .. } if b == *binder));
    assert!(free_binders(elaborated.resolved()).is_empty());
}

#[test]
fn a_quotation_binder_is_alpha_invariant_and_an_executable_one_is_not() {
    let store = Store::new();

    assert_eq!(
        term(&store, "term { fn(x) => x + y }"),
        term(&store, "term { fn(z) => z + y }"),
        "a symbolic binder carries no name"
    );
    assert_eq!(term(&store, "term { fn(x) => x }"), term(&store, "term { fn(z) => z }"));

    let one = resolve(&store, "fn(x) => x").into_resolved();
    let other = resolve(&store, "fn(z) => z").into_resolved();
    assert_ne!(one, other, "an executable binder keeps the name it was written with");
}

#[test]
fn a_quotation_binder_shadows_inside_the_term_too() {
    let store = Store::new();
    assert_eq!(
        term(&store, "term { fn(x) => fn(x) => x }"),
        term(&store, "term { fn(a) => fn(b) => b }"),
    );
    assert_ne!(
        term(&store, "term { fn(x) => fn(x) => x }"),
        term(&store, "term { fn(a) => fn(b) => a }"),
    );
}

// Without an unquote form, a name a quotation does not bind is a free symbol,
// whatever an enclosing scope does with it.
#[test]
fn a_quotation_does_not_read_the_lexical_environment() {
    let store = Store::new();
    let bound = resolve(&store, "let y = 1 in term { y }").into_resolved();
    let Resolved::Let { body, .. } = bound else {
        panic!("a let elaborates to a let");
    };
    let Resolved::Quote { term: quoted, .. } = *body else {
        panic!("the body is the quotation");
    };
    assert_eq!(quoted, store.symbol("y"));
    assert_eq!(quoted, term(&store, "term { y }"));
}

#[test]
fn a_quoted_body_reaches_the_store_through_layer_a() {
    let store = Store::new();
    assert_eq!(term(&store, "term { x + 2 }"), term(&store, "term { 2 + x }"));
    assert_eq!(term(&store, "term { x - x }"), term(&store, "term { 0 }"));
    assert_eq!(term(&store, "term { x * x }"), term(&store, "term { x ^ 2 }"));
    assert_eq!(term(&store, "term { -x }"), term(&store, "term { 0 - x }"));
}

#[test]
fn a_quotation_and_a_closure_are_two_paths_out_of_one_written_form() {
    let store = Store::new();
    let quoted = resolve(&store, "term { fn(x) => x + y }").into_resolved();
    let executable = resolve(&store, "fn(x) => x + y").into_resolved();

    assert!(matches!(quoted, Resolved::Quote { .. }));
    assert!(matches!(executable, Resolved::Lambda { .. }));
}

#[test]
fn every_resolved_node_says_where_it_came_from() {
    let store = Store::new();
    let source = "x + y";
    let tree = parse(SourceText::from(source));
    let elaborated = elaborate(&store, &lower(&tree)).expect("nothing was cancelled");

    let Resolved::Call { callee, arguments, origin } = elaborated.resolved() else {
        panic!("infix elaborates to a call");
    };
    assert_eq!(tree.source().slice(origin.span()), Some("x + y"));
    assert_eq!(
        tree.source().slice(callee.origin().span()),
        Some("x + y"),
        "the operator's origin is the expression it came from, not the token"
    );
    assert_eq!(tree.source().slice(arguments[0].origin().span()), Some("x"));
    assert_eq!(tree.source().slice(arguments[1].origin().span()), Some("y"));
}

#[test]
fn origin_survives_every_form_in_the_slice() {
    let store = Store::new();
    let elaborated = resolve(&store, "let f = fn(x) => x + term { y } in f(2, 1.5)");

    let mut origins = Vec::new();
    collect_origins(elaborated.resolved(), &mut origins);
    assert!(origins.len() >= 10, "the walk reached {} nodes", origins.len());
    assert!(
        origins.iter().all(|origin| matches!(origin, Origin::Source(_))),
        "nothing in well-formed source is recovered"
    );
    assert!(elaborated.diagnostics().is_empty());
}

fn collect_origins(node: &Resolved<'_>, out: &mut Vec<Origin>) {
    out.push(node.origin());
    match node {
        Resolved::Call { callee, arguments, .. } => {
            collect_origins(callee, out);
            for argument in arguments {
                collect_origins(argument, out);
            }
        }
        Resolved::Let { value, body, .. } => {
            collect_origins(value, out);
            collect_origins(body, out);
        }
        Resolved::Lambda { body, .. } => collect_origins(body, out),
        Resolved::Literal { .. }
        | Resolved::Local { .. }
        | Resolved::Global { .. }
        | Resolved::Quote { .. }
        | Resolved::Error { .. } => {}
    }
}

#[test]
fn a_decimal_is_a_runtime_literal_and_has_no_symbolic_form_yet() {
    let store = Store::new();

    let executable = resolve(&store, "1.50");
    assert!(executable.diagnostics().is_empty());
    assert!(matches!(
        executable.resolved(),
        Resolved::Literal { value: Literal::Decimal(text), .. } if &**text == "1.50"
    ));

    let quoted = resolve(&store, "term { 1.50 }");
    assert!(matches!(quoted.resolved(), Resolved::Error { .. }));
    assert_eq!(quoted.diagnostics().len(), 1);
}

#[test]
fn an_integer_literal_keeps_its_value_and_loses_its_separators() {
    let store = Store::new();
    assert!(matches!(
        resolve(&store, "1_000").resolved(),
        Resolved::Literal { value: Literal::Integer(1_000), .. }
    ));
    assert_eq!(term(&store, "term { 1_000 }"), store.int(1000).expect("1000 is small"));
}

#[test]
fn what_the_slice_does_not_cover_is_reported_rather_than_guessed() {
    let store = Store::new();
    for source in ["term { let a = 1 in a }", "term { term { x } }"] {
        let elaborated = resolve(&store, source);
        assert!(matches!(elaborated.resolved(), Resolved::Error { .. }), "{source:?}");
        assert_eq!(elaborated.diagnostics().len(), 1, "{source:?}");
    }
}

// Elaboration is total in the sense the parser is: malformed input reaches it,
// and it produces a tree rather than a panic.
#[test]
fn malformed_input_elaborates() {
    let store = Store::new();
    for source in ["", "f(x", "let", "x +", "term {", "@#$", "fn(x) x"] {
        let elaborated = resolve(&store, source);
        collect_origins(elaborated.resolved(), &mut Vec::new());
    }
}

// D22's channel reaches elaboration, because building a quoted term is
// construction and construction is what the token stops.
#[test]
fn cancellation_stops_a_quotation() {
    let store = Store::new();
    let token = CancelToken::new();
    token.cancel();
    store.set_cancel(Some(token));

    let syntax = lower(&parse(SourceText::from("term { x + 2 }")));
    assert_eq!(elaborate(&store, &syntax), Err(Cancelled));

    store.set_cancel(None);
    assert!(elaborate(&store, &syntax).is_ok());
}
