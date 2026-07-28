//! The resolved executable form: what elaboration produces and the compiler
//! reads.
//!
//! Names are gone from the semantics here and kept only as hints (D6). An
//! occurrence is a [`Resolved::Local`] carrying the [`BinderId`] that binds it
//! or a [`Resolved::Global`] carrying a name still to be looked up, and a
//! lambda carries the set of enclosing binders its body reaches.

use vieta_store::ExprId;
use vieta_syntax::Origin;

/// A lexical binder, distinct from every other binder in one elaboration.
///
/// Two spellings of one program get different ids, which is what lets the
/// executable form keep source identity while the symbolic form throws it away.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BinderId(pub u32);

/// A number, as elaboration read it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Literal {
    /// An exact integer.
    Integer(i64),
    /// A decimal, kept as written. Which runtime value it becomes is a
    /// representation question D34 leaves open.
    Decimal(Box<str>),
}

/// A binding a closure copies out of an enclosing scope.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Capture {
    /// The enclosing binder being copied.
    pub binder: BinderId,
    /// Its source name, kept as a reconstruction hint (D6).
    pub name: Box<str>,
}

/// A resolved expression.
///
/// The lifetime is the store's, because a quotation holds an interned term and
/// D8's rule about ids across a safepoint applies to whatever holds one.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Resolved<'s> {
    /// A number.
    Literal {
        /// What was written.
        value: Literal,
        /// Where it came from.
        origin: Origin,
    },
    /// An occurrence of a lexically bound name.
    Local {
        /// The binder it resolved to.
        binder: BinderId,
        /// Where it came from.
        origin: Origin,
    },
    /// An occurrence of a name no enclosing binder claims.
    ///
    /// Elaboration settles that the name is not lexical and stops there. Which
    /// definition it reaches is a world question (D20), and whether the call
    /// becomes a static, world-bound, or dynamic instruction is what the
    /// provisional bytecode is for (D38).
    Global {
        /// The name as written.
        name: Box<str>,
        /// Where it came from.
        origin: Origin,
    },
    /// A callee applied to arguments.
    ///
    /// Infix and prefix operators are calls by the time they get here, which is
    /// one of the surface constructs that needs no instruction of its own.
    Call {
        /// What is being called.
        callee: Box<Resolved<'s>>,
        /// The arguments, in order.
        arguments: Vec<Resolved<'s>>,
        /// Where it came from. For an operator this is the whole expression,
        /// not the operator token.
        origin: Origin,
    },
    /// A non-recursive lexical binding.
    Let {
        /// The binder it introduces.
        binder: BinderId,
        /// The bound name, as a reconstruction hint.
        name: Box<str>,
        /// What the binder is bound to, resolved outside its own scope.
        value: Box<Resolved<'s>>,
        /// The body, resolved inside it.
        body: Box<Resolved<'s>>,
        /// Where it came from.
        origin: Origin,
    },
    /// A lambda, which becomes code and a closure rather than a term (D6).
    Lambda {
        /// The parameter's binder.
        binder: BinderId,
        /// The parameter's name, as a reconstruction hint.
        name: Box<str>,
        /// What the closure copies from enclosing scopes, in the order the
        /// body first reaches each one.
        captures: Vec<Capture>,
        /// The body, resolved inside the parameter's scope.
        body: Box<Resolved<'s>>,
        /// Where it came from.
        origin: Origin,
    },
    /// A quotation, whose body was built in the store instead (D6).
    Quote {
        /// The interned term.
        term: ExprId<'s>,
        /// Where it came from.
        origin: Origin,
    },
    /// An expression that did not elaborate.
    Error {
        /// Where it came from.
        origin: Origin,
    },
}

impl Resolved<'_> {
    /// Where this node came from.
    pub fn origin(&self) -> Origin {
        match self {
            Resolved::Literal { origin, .. }
            | Resolved::Local { origin, .. }
            | Resolved::Global { origin, .. }
            | Resolved::Call { origin, .. }
            | Resolved::Let { origin, .. }
            | Resolved::Lambda { origin, .. }
            | Resolved::Quote { origin, .. }
            | Resolved::Error { origin } => *origin,
        }
    }
}

/// The binders this expression uses and does not itself bind, in the order a
/// traversal first reaches them.
///
/// This is what a lambda's capture set is computed from, and it reads a nested
/// lambda's stored captures rather than descending into it, so the two answers
/// come from one definition.
pub fn free_binders(node: &Resolved<'_>) -> Vec<BinderId> {
    let mut free = Vec::new();
    collect_free(node, &mut free);
    free
}

fn collect_free(node: &Resolved<'_>, free: &mut Vec<BinderId>) {
    match node {
        Resolved::Local { binder, .. } => push_once(free, *binder),
        Resolved::Call { callee, arguments, .. } => {
            collect_free(callee, free);
            for argument in arguments {
                collect_free(argument, free);
            }
        }
        Resolved::Let { binder, value, body, .. } => {
            collect_free(value, free);
            let mut inner = Vec::new();
            collect_free(body, &mut inner);
            for reached in inner {
                if reached != *binder {
                    push_once(free, reached);
                }
            }
        }
        Resolved::Lambda { captures, .. } => {
            for capture in captures {
                push_once(free, capture.binder);
            }
        }
        Resolved::Literal { .. }
        | Resolved::Global { .. }
        | Resolved::Quote { .. }
        | Resolved::Error { .. } => {}
    }
}

fn push_once(free: &mut Vec<BinderId>, binder: BinderId) {
    if !free.contains(&binder) {
        free.push(binder);
    }
}
