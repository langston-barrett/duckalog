use fxhash::{FxBuildHasher, FxHashMap as HashMap, FxHashSet as HashSet};

use crate::ast::{Ast, Const, Rel, Rule};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, thiserror::Error)]
pub enum Error {
    #[error("relation `{relation}` used with multiple arities: `{arity1}`, `{arity2}`")]
    Arity {
        relation: Rel,
        arity1: usize,
        arity2: usize,
    },
    // TODO(lb, low): improve error
    #[error("ungrounded variable")]
    Ungrounded,
}

/// Mid-level IR.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mir {
    facts: HashMap<Rel, HashSet<Vec<Const>>>,
    // TODO: Make this a Vec with invariant that its a set
    /// Invariant: Each [`Rule`] has a non-empty body
    rules: HashSet<Rule>,
}

impl Mir {
    pub fn add_fact(&mut self, rel: &Rel, consts: Vec<Const>) {
        self.facts
            .entry(rel.clone())
            .or_insert_with(|| HashSet::with_capacity_and_hasher(1, FxBuildHasher::default()))
            .insert(consts);
    }

    pub fn arities(&self) -> HashMap<Rel, usize> {
        let mut arities = HashMap::with_capacity_and_hasher(
            self.facts.len(), // lower bound
            FxBuildHasher::default(),
        );
        for (rel, consts) in &self.facts {
            arities.insert(rel.clone(), consts.iter().next().unwrap().len());
        }
        for rule in &self.rules {
            let rel = &rule.head.rel;
            match arities.get(rel).copied() {
                None => {
                    arities.insert(rel.clone(), rule.head.terms.len());
                }
                Some(arity) => {
                    debug_assert_eq!(arity, rule.head.terms.len());
                }
            }
        }
        debug_assert!(arities.len() >= self.facts.len());
        arities
    }

    pub fn clear_facts(&mut self) {
        self.facts.clear()
    }

    pub fn facts(&self) -> impl Iterator<Item = (&Rel, impl Iterator<Item = &Vec<Const>>)> {
        self.facts.iter().map(|(rel, consts)| (rel, consts.iter()))
    }

    pub fn new(ast: Ast) -> Result<Self, Error> {
        let prog = Self::new_unchecked(ast)?;
        prog.valid()?;
        Ok(prog)
    }

    pub fn new_unchecked(ast: Ast) -> Result<Self, Error> {
        let mut facts =
            HashMap::with_capacity_and_hasher(ast.rules.len(), FxBuildHasher::default());
        let mut rules =
            HashSet::with_capacity_and_hasher(ast.rules.len(), FxBuildHasher::default());
        for rule in ast.rules {
            if rule.is_fact() {
                let fact = match rule.head.ground() {
                    None => return Err(Error::Ungrounded),
                    Some(f) => f,
                };
                facts
                    .entry(fact.rel)
                    .or_insert_with(|| {
                        HashSet::with_capacity_and_hasher(1, FxBuildHasher::default())
                    })
                    .insert(fact.terms);
            } else {
                rules.insert(rule);
            }
        }
        facts.shrink_to_fit();
        rules.shrink_to_fit();
        Ok(Self { facts, rules })
    }

    pub fn rules(&self) -> impl Iterator<Item = &Rule> {
        self.rules.iter()
    }

    pub fn valid(&self) -> Result<(), Error> {
        // TODO(lb, low): Actually check some things
        Ok(())
    }
}
