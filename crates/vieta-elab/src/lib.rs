//! Elaboration: the resolved layer between the parser and the compiler (D38).
//!
//! Surface syntax says what was written. This crate says what it means
//! lexically: which occurrence is bound by which binder, what a closure copies
//! from around it, and which names are left for a world to resolve. The
//! instruction set is meant to be read off this form rather than off the
//! grammar, which is why it exists before any bytecode does.
//!
//! Binding takes two paths, and which one a form takes is the context it sits
//! in (D6). A lambda in ordinary position becomes code and a closure, with the
//! binder resolved and the name kept only as a hint:
//!
//! ```
//! use vieta_elab::{Resolved, elaborate};
//! use vieta_store::Store;
//! use vieta_syntax::{SourceText, lower, parse};
//!
//! let store = Store::new();
//! let syntax = lower(&parse(SourceText::from("fn(x) => x + y")));
//! let elaborated = elaborate(&store, &syntax)?;
//!
//! let Resolved::Lambda { binder, body, .. } = elaborated.resolved() else {
//!     panic!("a lambda elaborates to a lambda");
//! };
//! let Resolved::Call { arguments, .. } = &**body else {
//!     panic!("infix is a call by now");
//! };
//! assert_eq!(arguments[0], Resolved::Local { binder: *binder, origin: arguments[0].origin() });
//! assert!(matches!(arguments[1], Resolved::Global { .. }), "y is nobody's local");
//! # Ok::<(), vieta_store::Cancelled>(())
//! ```
//!
//! The same lambda inside a quotation becomes a term instead, and a term's
//! binder carries no name, so two spellings of it are one `ExprId`:
//!
//! ```
//! # use vieta_elab::{Resolved, elaborate};
//! # use vieta_store::Store;
//! # use vieta_syntax::{SourceText, lower, parse};
//! # let store = Store::new();
//! let one = lower(&parse(SourceText::from("term { fn(x) => x + y }")));
//! let other = lower(&parse(SourceText::from("term { fn(z) => z + y }")));
//!
//! let (Resolved::Quote { term: one, .. }, Resolved::Quote { term: other, .. }) =
//!     (elaborate(&store, &one)?.into_resolved(), elaborate(&store, &other)?.into_resolved())
//! else {
//!     panic!("a quotation elaborates to a term");
//! };
//! assert_eq!(one, other);
//! # Ok::<(), vieta_store::Cancelled>(())
//! ```
//!
//! What this slice covers is literals, calls, `let`, lambda, quotation, and
//! origin propagation, which is the smallest arrangement in which a lexical
//! binder has to be represented for real (D38). Everything else it meets, it
//! reports.

mod elaborate;
mod quote;
mod resolved;

pub use elaborate::{ElabError, Elaboration, elaborate};
pub use resolved::{BinderId, Capture, Literal, Resolved, free_binders};
