//! The symbol table: a flat text arena plus an index into it.
//!
//! An entry is keyed on a module and a name, which together identify an
//! operator (D36), and it carries that operator's canonical signature. The
//! signature is fixed once and never changes: either a declaration fixes it, or
//! the first use of the symbol as a head fixes it to the empty one.

use crate::hash::{hash_bytes, mix};
use crate::operator::{RawSignature, SignatureConflict};
use crate::probe::{Probe, ProbeTable};

#[derive(Clone, Copy)]
struct Span {
    offset: u32,
    len: u32,
    module: u32,
}

#[derive(Default)]
pub(crate) struct SymbolTable {
    text: String,
    spans: Vec<Span>,
    signatures: Vec<Option<RawSignature>>,
    index: ProbeTable,
}

fn key_hash(module: u32, name: &str) -> u32 {
    mix(hash_bytes(name.as_bytes()), module)
}

fn span_str<'a>(text: &'a str, spans: &[Span], entry: u32) -> &'a str {
    let span = spans[entry as usize];
    &text[span.offset as usize..(span.offset + span.len) as usize]
}

impl SymbolTable {
    pub(crate) fn intern(&mut self, module: u32, name: &str) -> u32 {
        let hash = key_hash(module, name);
        let probe = {
            let text = &self.text;
            let spans = &self.spans;
            let index = &self.index;
            index.probe(hash, |entry| {
                spans[entry as usize].module == module && span_str(text, spans, entry) == name
            })
        };
        match probe {
            Probe::Found(entry) => entry,
            Probe::Vacant(slot) => {
                let offset = self.text.len() as u32;
                self.text.push_str(name);
                let entry = self.spans.len() as u32;
                self.spans.push(Span { offset, len: name.len() as u32, module });
                self.signatures.push(None);
                self.index.occupy(slot, entry);
                if self.index.needs_grow() {
                    let text = &self.text;
                    let spans = &self.spans;
                    self.index.grow(|entry| {
                        key_hash(spans[entry as usize].module, span_str(text, spans, entry))
                    });
                }
                entry
            }
        }
    }

    pub(crate) fn name(&self, entry: u32) -> Option<&str> {
        if (entry as usize) < self.spans.len() {
            Some(span_str(&self.text, &self.spans, entry))
        } else {
            None
        }
    }

    pub(crate) fn module(&self, entry: u32) -> Option<u32> {
        self.spans.get(entry as usize).map(|span| span.module)
    }

    /// The signature fixed for this operator, or `None` while it is still open.
    pub(crate) fn signature(&self, entry: u32) -> Option<RawSignature> {
        self.signatures.get(entry as usize).copied().flatten()
    }

    /// Fix a signature, or report that a different one is already fixed.
    /// Fixing the same signature twice is not an error.
    pub(crate) fn fix(
        &mut self,
        entry: u32,
        signature: RawSignature,
    ) -> Result<(), SignatureConflict> {
        match self.signatures[entry as usize] {
            Some(existing) if existing == signature => Ok(()),
            Some(_) => Err(SignatureConflict),
            None => {
                self.signatures[entry as usize] = Some(signature);
                Ok(())
            }
        }
    }

    /// The signature fixed for this operator, fixing the empty one if it is
    /// still open. This is what using a symbol as a head does, and it is why a
    /// declaration afterwards conflicts.
    pub(crate) fn fix_empty(&mut self, entry: u32) -> RawSignature {
        let slot = &mut self.signatures[entry as usize];
        *slot.get_or_insert(RawSignature::EMPTY)
    }

    pub(crate) fn len(&self) -> usize {
        self.spans.len()
    }

    pub(crate) fn reserved_bytes(&self) -> usize {
        self.text.capacity()
            + self.spans.capacity() * size_of::<Span>()
            + self.signatures.capacity() * size_of::<Option<RawSignature>>()
            + self.index.heap_bytes()
    }

    pub(crate) fn used_bytes(&self) -> usize {
        self.text.len()
            + self.spans.len() * size_of::<Span>()
            + self.signatures.len() * size_of::<Option<RawSignature>>()
            + self.index.heap_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::SymbolTable;
    use crate::operator::RawSignature;

    const ROOT: u32 = 0;

    #[test]
    fn one_entry_per_name() {
        let mut table = SymbolTable::default();
        let x = table.intern(ROOT, "x");
        let y = table.intern(ROOT, "y");
        assert_ne!(x, y);
        assert_eq!(table.intern(ROOT, "x"), x);
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn the_module_is_part_of_the_key() {
        let mut table = SymbolTable::default();
        let core = table.intern(ROOT, "Plus");
        let user = table.intern(1, "Plus");
        assert_ne!(core, user);
        assert_eq!(table.name(core), table.name(user));
        assert_eq!(table.module(core), Some(ROOT));
        assert_eq!(table.module(user), Some(1));
        assert_eq!(table.intern(1, "Plus"), user);
    }

    #[test]
    fn names_come_back_intact() {
        let mut table = SymbolTable::default();
        let names = ["x", "Integrate", "\u{03b1}", "", "Plus"];
        let entries: Vec<u32> = names.iter().map(|n| table.intern(ROOT, n)).collect();
        for (entry, name) in entries.iter().zip(names) {
            assert_eq!(table.name(*entry), Some(name));
        }
        assert_eq!(table.name(entries.len() as u32), None);
    }

    #[test]
    fn a_prefix_is_not_its_extension() {
        let mut table = SymbolTable::default();
        let a = table.intern(ROOT, "Sin");
        let b = table.intern(ROOT, "Sinh");
        assert_ne!(a, b);
        assert_eq!(table.name(a), Some("Sin"));
        assert_eq!(table.name(b), Some("Sinh"));
    }

    #[test]
    fn many_names_survive_growth() {
        let mut table = SymbolTable::default();
        let names: Vec<String> = (0..4_000).map(|i| format!("sym{i}")).collect();
        let entries: Vec<u32> = names.iter().map(|n| table.intern(ROOT, n)).collect();
        assert_eq!(table.len(), names.len());
        for (entry, name) in entries.iter().zip(&names) {
            assert_eq!(table.name(*entry), Some(name.as_str()));
            assert_eq!(table.intern(ROOT, name), *entry);
        }
        assert_eq!(table.len(), names.len());
    }

    #[test]
    fn a_signature_is_open_until_it_is_fixed() {
        let mut table = SymbolTable::default();
        let f = table.intern(ROOT, "f");
        assert_eq!(table.signature(f), None);
        let associative = RawSignature { associative: true, ..RawSignature::EMPTY };
        assert!(table.fix(f, associative).is_ok());
        assert_eq!(table.signature(f), Some(associative));
    }

    #[test]
    fn fixing_the_same_signature_twice_is_allowed() {
        let mut table = SymbolTable::default();
        let f = table.intern(ROOT, "f");
        let associative = RawSignature { associative: true, ..RawSignature::EMPTY };
        assert!(table.fix(f, associative).is_ok());
        assert!(table.fix(f, associative).is_ok());
    }

    #[test]
    fn a_different_signature_conflicts() {
        let mut table = SymbolTable::default();
        let f = table.intern(ROOT, "f");
        assert!(table.fix(f, RawSignature { associative: true, ..RawSignature::EMPTY }).is_ok());
        assert!(table.fix(f, RawSignature { commutative: true, ..RawSignature::EMPTY }).is_err());
    }

    #[test]
    fn use_as_a_head_fixes_the_empty_signature() {
        let mut table = SymbolTable::default();
        let f = table.intern(ROOT, "f");
        assert_eq!(table.fix_empty(f), RawSignature::EMPTY);
        assert_eq!(table.signature(f), Some(RawSignature::EMPTY));
        assert!(table.fix(f, RawSignature { associative: true, ..RawSignature::EMPTY }).is_err());
    }

    #[test]
    fn a_fixed_signature_survives_use_as_a_head() {
        let mut table = SymbolTable::default();
        let f = table.intern(ROOT, "f");
        let associative = RawSignature { associative: true, ..RawSignature::EMPTY };
        assert!(table.fix(f, associative).is_ok());
        assert_eq!(table.fix_empty(f), associative);
    }
}
