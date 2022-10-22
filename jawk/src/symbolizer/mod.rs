use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::{Weak, Rc};
use hashbrown::HashSet;

#[derive(Clone, Debug)]
struct WeaklyHeldStr {
    w: Weak<String>,
}

impl WeaklyHeldStr {
    fn upgrade(self) -> Option<Symbol> {
        match self.w.upgrade() {
            None => None,
            Some(sym) => Some(Symbol { sym })
        }
    }
}

impl PartialEq<Self> for WeaklyHeldStr {
    // This is string == string equality
    // Slow equality
    fn eq(&self, other: &Self) -> bool {
        if let Some(rc1) = self.w.upgrade() {
            if let Some(rc2) = other.w.upgrade() {
                return rc1 == rc2;
            }
        }
        false
    }
}

impl Eq for WeaklyHeldStr {}

impl Hash for WeaklyHeldStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(rc) = self.w.upgrade() {
            rc.hash(state);
        } else {
            state.write_i8(0);
        }
    }
}

pub struct Symbolizer {
    known: HashSet<WeaklyHeldStr>,
}

#[derive(Clone, Debug, Hash, PartialOrd, Ord)]
pub struct Symbol {
    pub sym: Rc<String>,
}

impl Symbol {
    pub fn to_str(&self) -> &str {
        self.sym.as_str()
    }
    #[allow(dead_code)]
    pub fn as_bytes(&self) -> &[u8] {
        self.sym.as_bytes()
    }
}

impl PartialEq<Self> for Symbol {
    // This is ptr equality. We assume that if two symbols have the
    // same pty they are the same string. This is the invariant of
    // the symbolizer. If you mix symbols from two symbolizer they will never be eq.
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.sym, &other.sym)
    }
}

impl Eq for Symbol {}

impl Display for Symbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}", self.sym))
    }
}

impl Symbolizer {
    pub fn new() -> Self { Symbolizer { known: HashSet::with_capacity(8) } }
    pub fn get<T: Into<String>>(&mut self, str: T) -> Symbol {
        let sym = Rc::new(str.into());
        let s: Symbol = Symbol { sym: sym.clone() };
        let weakly_held = WeaklyHeldStr { w: Rc::downgrade(&sym) };
        if let Some(existing) = self.known.get(&weakly_held) {
            let upgraded = existing.clone().upgrade();
            if let Some(existing_symbol) = upgraded {
                return existing_symbol.clone();
            }
        }
        self.known.insert(weakly_held);
        s
    }
}

#[test]
fn test() {
    let mut symbolizer = Symbolizer::new();
    {
        let abc1 = String::from("abc");
        let abc2 = String::from("abc");
        let other = String::from("other");
        let sym1 = symbolizer.get(abc1);
        let sym2 = symbolizer.get(abc2);
        let sym_other = symbolizer.get(other);
        assert_eq!(sym1, sym2);
        assert_ne!(sym_other, sym1);
        assert_ne!(sym1, sym_other);
    };
    let a = String::from("yyz");
    let aa = symbolizer.get(a);
    {
        let a2 = String::from("yyz");
        let aaa = symbolizer.get(a2);
        assert_eq!(aa, aaa);
    }
}