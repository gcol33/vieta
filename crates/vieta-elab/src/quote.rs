//! The symbolic path: a quotation becomes an alpha-invariant store term.
//!
//! A binder here is term data rather than code, so its name does not survive:
//! an occurrence becomes its de Bruijn index into the enclosing binders, and
//! `term { fn(x) => x + y }` and `term { fn(z) => z + y }` reach one `ExprId`.
//!
//! Two choices in this file are provisional and D6 says so. The symbolic
//! binder's encoding is open, and de Bruijn indices are what this slice builds
//! against. And a quotation captures no lexical environment, since a name the
//! quotation does not bind becomes a free symbol whether or not an enclosing
//! scope binds it; splicing needs an unquote form that no entry has decided on.

use vieta_store::{Cancelled, ExprId, Store};
use vieta_syntax::{BinaryOp, Origin, Syntax, UnaryOp};

use crate::elaborate::ElabError;

/// The head of a symbolic binder, and the head of an occurrence bound by one.
///
/// Neither name can be written in source, since an identifier admits no `$`.
const LAMBDA: &str = "$Lambda";
const BOUND: &str = "$Bound";

/// Build a quotation body as a term, recording why if it cannot be built.
pub(crate) fn quote<'s>(
    store: &'s Store,
    syntax: &Syntax,
    diagnostics: &mut Vec<ElabError>,
) -> Result<Option<ExprId<'s>>, Cancelled> {
    Quoter { store, binders: Vec::new(), diagnostics }.term(syntax)
}

struct Quoter<'s, 'd> {
    store: &'s Store,
    /// The enclosing symbolic binders, innermost last, holding the names an
    /// occurrence is matched against before its index replaces them.
    binders: Vec<Box<str>>,
    diagnostics: &'d mut Vec<ElabError>,
}

impl<'s> Quoter<'s, '_> {
    fn term(&mut self, syntax: &Syntax) -> Result<Option<ExprId<'s>>, Cancelled> {
        let origin = syntax.origin();
        match syntax {
            Syntax::Number { text, .. } => Ok(self.number(text, origin)),
            Syntax::Name { text, .. } => self.name(text).map(Some),
            Syntax::Unary { op: UnaryOp::Negate, operand, .. } => {
                let Some(operand) = self.term(operand)? else {
                    return Ok(None);
                };
                self.negate(operand).map(Some)
            }
            Syntax::Binary { op, left, right, .. } => {
                let Some(left) = self.term(left)? else {
                    return Ok(None);
                };
                let Some(right) = self.term(right)? else {
                    return Ok(None);
                };
                self.operator(*op, left, right).map(Some)
            }
            Syntax::Call { callee, arguments, .. } => {
                let Some(head) = self.term(callee)? else {
                    return Ok(None);
                };
                let mut built = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    let Some(argument) = self.term(argument)? else {
                        return Ok(None);
                    };
                    built.push(argument);
                }
                self.store.app(head, &built).map(Some)
            }
            Syntax::Lambda { parameter, body, .. } => {
                self.binders.push(parameter.clone());
                let body = self.term(body);
                self.binders.pop();
                let Some(body) = body? else {
                    return Ok(None);
                };
                let head = self.store.symbol(LAMBDA);
                self.store.app(head, &[body]).map(Some)
            }
            Syntax::Let { .. } => {
                Ok(self.refuse(origin, "a `let` in a quotation is not in this slice"))
            }
            Syntax::Quote { .. } => {
                Ok(self.refuse(origin, "a quotation in a quotation is not in this slice"))
            }
            // The parser reported this one already.
            Syntax::Error { .. } => Ok(None),
        }
    }

    /// An occurrence: its index when a quotation binder claims it, and a free
    /// symbol when none does.
    fn name(&mut self, text: &str) -> Result<ExprId<'s>, Cancelled> {
        match self.binders.iter().rposition(|name| &**name == text) {
            Some(level) => self.bound(self.binders.len() - 1 - level),
            None => Ok(self.store.symbol(text)),
        }
    }

    fn bound(&mut self, index: usize) -> Result<ExprId<'s>, Cancelled> {
        let head = self.store.symbol(BOUND);
        let index = self.small(index as i64);
        self.store.app(head, &[index])
    }

    fn number(&mut self, text: &str, origin: Origin) -> Option<ExprId<'s>> {
        if text.contains('.') {
            return self.refuse(origin, "a decimal literal has no symbolic form in this slice");
        }
        let digits: String = text.chars().filter(|ch| *ch != '_').collect();
        let built = digits.parse::<i64>().ok().and_then(|value| self.store.int(value));
        match built {
            Some(id) => Some(id),
            None => self.refuse(
                origin,
                "the integer literal needs the large-integer side table, which arrives with M1",
            ),
        }
    }

    /// Layer A has three operators (`layer-a.md` §7), so subtraction, division,
    /// and negation reach the store as the forms it does have.
    fn operator(
        &mut self,
        op: BinaryOp,
        left: ExprId<'s>,
        right: ExprId<'s>,
    ) -> Result<ExprId<'s>, Cancelled> {
        match op {
            BinaryOp::Add => self.store.app(self.store.plus(), &[left, right]),
            BinaryOp::Subtract => {
                let negated = self.negate(right)?;
                self.store.app(self.store.plus(), &[left, negated])
            }
            BinaryOp::Multiply => self.store.app(self.store.times(), &[left, right]),
            BinaryOp::Divide => {
                let minus_one = self.small(-1);
                let inverse = self.store.app(self.store.power(), &[right, minus_one])?;
                self.store.app(self.store.times(), &[left, inverse])
            }
            BinaryOp::Power => self.store.app(self.store.power(), &[left, right]),
        }
    }

    fn negate(&mut self, operand: ExprId<'s>) -> Result<ExprId<'s>, Cancelled> {
        let minus_one = self.small(-1);
        self.store.app(self.store.times(), &[minus_one, operand])
    }

    fn small(&self, value: i64) -> ExprId<'s> {
        self.store.int(value).expect("the constants this file builds with are small")
    }

    fn refuse(&mut self, origin: Origin, message: &str) -> Option<ExprId<'s>> {
        self.diagnostics.push(ElabError { origin, message: message.to_owned() });
        None
    }
}
