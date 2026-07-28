//! The elaborator: surface syntax in, resolved executable structure out.
//!
//! It is total in the same sense the parser is. An expression it cannot resolve
//! becomes [`Resolved::Error`] and a diagnostic, and the rest of the program
//! still elaborates, so a caller always gets a tree to look at.

use vieta_store::{Cancelled, Store};
use vieta_syntax::{BinaryOp, Origin, Syntax, UnaryOp};

use crate::quote::quote;
use crate::resolved::{BinderId, Capture, Literal, Resolved, free_binders};

/// Something elaboration could not resolve.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ElabError {
    /// Where the problem is.
    pub origin: Origin,
    /// What went wrong, in one phrase.
    pub message: String,
}

/// A resolved program and everything that went wrong resolving it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Elaboration<'s> {
    resolved: Resolved<'s>,
    diagnostics: Vec<ElabError>,
}

impl<'s> Elaboration<'s> {
    /// The resolved form.
    pub fn resolved(&self) -> &Resolved<'s> {
        &self.resolved
    }

    /// Everything that went wrong, in the order elaboration reached it.
    pub fn diagnostics(&self) -> &[ElabError] {
        &self.diagnostics
    }

    /// The resolved form, giving up the diagnostics.
    pub fn into_resolved(self) -> Resolved<'s> {
        self.resolved
    }
}

/// Resolve surface syntax against a store.
///
/// The store is needed because a quotation builds a term while elaborating
/// (D6), and it is the only thing here that can be cancelled: building a term
/// goes through Layer A, which honours the store's cancellation token (D22).
pub fn elaborate<'s>(store: &'s Store, syntax: &Syntax) -> Result<Elaboration<'s>, Cancelled> {
    let mut elaborator =
        Elaborator { store, scope: Vec::new(), names: Vec::new(), diagnostics: Vec::new() };
    let resolved = elaborator.expression(syntax)?;
    Ok(Elaboration { resolved, diagnostics: elaborator.diagnostics })
}

struct Elaborator<'s> {
    store: &'s Store,
    /// The bindings in scope, innermost last, which is what makes the last
    /// binder of a name the one an occurrence resolves to.
    scope: Vec<(Box<str>, BinderId)>,
    /// Every binder's source name, indexed by binder.
    names: Vec<Box<str>>,
    diagnostics: Vec<ElabError>,
}

impl<'s> Elaborator<'s> {
    fn expression(&mut self, syntax: &Syntax) -> Result<Resolved<'s>, Cancelled> {
        let origin = syntax.origin();
        let resolved = match syntax {
            Syntax::Number { text, .. } => self.number(text, origin),
            Syntax::Name { text, .. } => self.name(text, origin),
            Syntax::Unary { op, operand, .. } => {
                let operand = self.expression(operand)?;
                call(unary_name(*op), vec![operand], origin)
            }
            Syntax::Binary { op, left, right, .. } => {
                let left = self.expression(left)?;
                let right = self.expression(right)?;
                call(binary_name(*op), vec![left, right], origin)
            }
            Syntax::Call { callee, arguments, .. } => {
                let callee = self.expression(callee)?;
                let mut resolved = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    resolved.push(self.expression(argument)?);
                }
                Resolved::Call { callee: Box::new(callee), arguments: resolved, origin }
            }
            // The value is resolved before the binder exists, which is what
            // makes the binding non-recursive.
            Syntax::Let { name, value, body, .. } => {
                let value = self.expression(value)?;
                let binder = self.bind(name);
                let body = self.expression(body);
                self.scope.pop();
                Resolved::Let {
                    binder,
                    name: name.clone(),
                    value: Box::new(value),
                    body: Box::new(body?),
                    origin,
                }
            }
            Syntax::Lambda { parameter, body, .. } => {
                let binder = self.bind(parameter);
                let body = self.expression(body);
                self.scope.pop();
                let body = body?;
                Resolved::Lambda {
                    binder,
                    name: parameter.clone(),
                    captures: self.captures(&body, binder),
                    body: Box::new(body),
                    origin,
                }
            }
            Syntax::Quote { body, .. } => match quote(self.store, body, &mut self.diagnostics)? {
                Some(term) => Resolved::Quote { term, origin },
                None => Resolved::Error { origin },
            },
            // The parser reported this one already.
            Syntax::Error { .. } => Resolved::Error { origin },
        };
        Ok(resolved)
    }

    fn name(&mut self, text: &str, origin: Origin) -> Resolved<'s> {
        match self.scope.iter().rev().find(|(name, _)| &**name == text) {
            Some((_, binder)) => Resolved::Local { binder: *binder, origin },
            None => Resolved::Global { name: text.into(), origin },
        }
    }

    fn number(&mut self, text: &str, origin: Origin) -> Resolved<'s> {
        if text.contains('.') {
            return Resolved::Literal { value: Literal::Decimal(text.into()), origin };
        }
        let digits: String = text.chars().filter(|ch| *ch != '_').collect();
        match digits.parse::<i64>() {
            Ok(value) => Resolved::Literal { value: Literal::Integer(value), origin },
            Err(_) => {
                self.diagnostics.push(ElabError {
                    origin,
                    message: "the integer literal does not fit in 64 bits".to_owned(),
                });
                Resolved::Error { origin }
            }
        }
    }

    fn bind(&mut self, name: &str) -> BinderId {
        let binder = BinderId(self.names.len() as u32);
        self.names.push(name.into());
        self.scope.push((name.into(), binder));
        binder
    }

    /// What a closure copies: the binders its body reaches and does not bind,
    /// which is free-variable analysis with the parameter removed.
    fn captures(&self, body: &Resolved<'s>, parameter: BinderId) -> Vec<Capture> {
        free_binders(body)
            .into_iter()
            .filter(|binder| *binder != parameter)
            .map(|binder| Capture { binder, name: self.names[binder.0 as usize].clone() })
            .collect()
    }
}

/// An operator applied to its operands. The origin is the whole expression,
/// since the operator token is the part elaboration is removing.
fn call<'s>(operator: &str, arguments: Vec<Resolved<'s>>, origin: Origin) -> Resolved<'s> {
    Resolved::Call {
        callee: Box::new(Resolved::Global { name: operator.into(), origin }),
        arguments,
        origin,
    }
}

fn binary_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Power => "^",
    }
}

/// Prefix minus gets a name of its own, so that arity is not what tells the two
/// forms of `-` apart.
fn unary_name(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Negate => "negate",
    }
}
