use std::collections::HashMap;
use std::fmt::Display;

use duckdb::types::{ToSqlOutput, ValueRef};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum Error {
    #[error("relation `{relation}` used with multiple arities: `{arity1}`, `{arity2}`")]
    Arity {
        relation: Rel,
        arity1: usize,
        arity2: usize,
    },
    #[error("ungrounded variable `{var}` in rule `{rule}`")]
    Ungrounded { rule: Rule, var: Var },
}

// TODO(lb, low): other types
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Const(String);

impl Display for Const {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl duckdb::ToSql for Const {
    fn to_sql(&self) -> duckdb::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(ValueRef::Text(self.0.as_bytes())))
    }
}

impl Const {
    pub fn new(s: String) -> Option<Self> {
        if Self::valid(&s) {
            Some(Self(s))
        } else {
            None
        }
    }

    pub fn new_unchecked(s: String) -> Self {
        Self(s)
    }

    pub fn valid(s: &str) -> bool {
        match s.chars().next() {
            None => false,
            Some(c) => c.is_lowercase(),
        }
    }
}

// TODO(lb): check for uppercase
// TODO(lb, low): small string optimization, or:
// TODO(lb, low): replace with indices into a hash-cons table
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Var(String);

impl Display for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Var {
    pub fn new(s: String) -> Option<Self> {
        if Self::valid(&s) {
            Some(Self(s))
        } else {
            None
        }
    }

    pub fn new_unchecked(s: String) -> Self {
        Self(s)
    }

    pub fn valid(s: &str) -> bool {
        match s.chars().next() {
            None => false,
            Some(c) => c.is_uppercase(),
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Term {
    Const(Const),
    Var(Var),
}

impl Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::Const(c) => write!(f, "{}", c),
            Term::Var(v) => write!(f, "{}", v),
        }
    }
}

// TODO(lb, low): small string optimization, or:
// TODO(lb, low): replace with indices into a hash-cons table
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Rel(String);

impl Display for Rel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Rel {
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Atom {
    pub(crate) rel: Rel,
    pub(crate) terms: Vec<Term>, // TODO(lb, low): small vec optimization
}

impl Display for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}(", self.rel)?;
        let mut iter = self.terms.iter();
        if let Some(term) = iter.next() {
            write!(f, "{}", term)?;
            for term in iter {
                write!(f, ", {}", term)?;
            }
        }
        write!(f, ")")
    }
}

impl Atom {
    pub fn new(rel: Rel, terms: Vec<Term>) -> Self {
        Self { rel, terms }
    }

    pub fn ground(self) -> Option<GroundAtom> {
        let mut consts = Vec::with_capacity(self.terms.len());
        for term in self.terms {
            if let Term::Const(c) = term {
                consts.push(c);
            } else {
                return None;
            }
        }
        Some(GroundAtom::new(self.rel, consts))
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GroundAtom {
    pub(crate) rel: Rel,
    pub(crate) terms: Vec<Const>, // TODO(lb, low): small vec optimization
}

impl GroundAtom {
    pub fn new(rel: Rel, terms: Vec<Const>) -> Self {
        Self { rel, terms }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Rule {
    pub(crate) head: Atom,
    pub(crate) body: Vec<Atom>, // TODO(lb, low): small vec optimization
}

impl Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} :- ", self.head)?;
        let mut iter = self.body.iter();
        if let Some(atom) = iter.next() {
            write!(f, "{}", atom)?;
            for atom in iter {
                write!(f, ", {}", atom)?;
            }
        }
        write!(f, ".")
    }
}

impl Rule {
    // TODO: range restriction check
    pub fn new(head: Atom, body: Vec<Atom>) -> Self {
        Self { head, body }
    }

    pub fn is_fact(&self) -> bool {
        self.body.is_empty()
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Ast {
    pub(crate) rules: Vec<Rule>,
}

struct Atoms<'a> {
    prog: &'a Ast,
    rule: usize,
    atom: usize,
}

impl<'a> Iterator for Atoms<'a> {
    type Item = &'a Atom;

    fn next(&mut self) -> Option<Self::Item> {
        match self.prog.rules.get(self.rule) {
            None => None,
            Some(rule) => {
                if self.atom == 0 {
                    if rule.body.is_empty() {
                        self.rule += 1;
                    } else {
                        self.atom += 1;
                    }
                    Some(&rule.head)
                } else {
                    let atom = self.atom - 1;
                    if rule.body.len() > atom + 1 {
                        self.atom += 1;
                    } else {
                        self.rule += 1;
                    }
                    Some(rule.body.get(atom).unwrap())
                }
            }
        }
    }
}

impl Ast {
    pub fn atoms(&self) -> impl Iterator<Item = &Atom> {
        Atoms {
            prog: self,
            rule: 0,
            atom: 0,
        }
    }

    pub fn arities(&self) -> HashMap<Rel, usize> {
        let mut arities = HashMap::with_capacity(self.rules.len() / 8); // just a guess
        for atom in self.atoms() {
            match arities.get(&atom.rel).copied() {
                None => {
                    arities.insert(atom.rel.clone(), atom.terms.len());
                }
                Some(arity) => {
                    debug_assert_eq!(arity, atom.terms.len())
                }
            }
        }
        arities
    }

    pub fn new(rules: Vec<Rule>) -> Result<Self, Error> {
        let prog = Self { rules };
        prog.valid()?;
        Ok(prog)
    }

    pub fn new_unchecked(rules: Vec<Rule>) -> Self {
        Self { rules }
    }

    pub fn valid(&self) -> Result<(), Error> {
        let mut arities = HashMap::with_capacity(self.rules.len() / 8); // just a guess
        let mut check = |atom: &Atom| match arities.get(&atom.rel).copied() {
            None => {
                arities.insert(atom.rel.clone(), atom.terms.len());
                Ok(())
            }
            Some(arity) => {
                if arity == atom.terms.len() {
                    Ok(())
                } else {
                    Err(Error::Arity {
                        relation: atom.rel.clone(),
                        arity1: arity,
                        arity2: atom.terms.len(),
                    })
                }
            }
        };
        for atom in self.atoms() {
            check(atom)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn null_atom() -> Atom {
        Atom::new(Rel::new(String::from("r")), Vec::new())
    }

    fn null_fact() -> Rule {
        Rule::new(null_atom(), Vec::new())
    }

    fn unary_atom() -> Atom {
        Atom::new(
            Rel::new(String::from("r")),
            vec![Term::Const(Const::new_unchecked(String::from("c")))],
        )
    }

    fn unary_fact() -> Rule {
        Rule::new(unary_atom(), Vec::new())
    }

    #[test]
    fn print_nullary_fact() {
        assert_eq!("r() :- .", &format!("{}", null_fact()))
    }

    #[test]
    fn print_unary_fact() {
        assert_eq!("r(c) :- .", &format!("{}", unary_fact()))
    }

    #[test]
    fn nullary_prog_ok() {
        let prog = Ast::new(vec![null_fact()]).unwrap();
        assert_eq!(
            Ast {
                rules: vec![null_fact()]
            },
            prog
        );
        assert_eq!(vec![&null_atom()], prog.atoms().collect::<Vec<_>>());
    }

    #[test]
    fn unary_prog_ok() {
        let prog = Ast::new(vec![unary_fact()]).unwrap();
        assert_eq!(
            Ast {
                rules: vec![unary_fact()]
            },
            prog
        );
        assert_eq!(vec![&unary_atom()], prog.atoms().collect::<Vec<_>>());
    }
}
