#[cfg(feature = "duckdb")]
use duckdb::{Connection, Result};
#[cfg(feature = "sqlite")]
use rusqlite::{Connection, Result};

use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};

use crate::ast::{Const, Rel, Rule, Term};
use crate::mir::Mir;

#[derive(Debug)]
pub struct Eval {
    conn: Connection,
    prog: Mir,
}

fn create_table(rel: &Rel, arity: usize) -> String {
    let mut attrs = Vec::with_capacity(arity);
    let mut indices = Vec::new();
    for i in 0..arity {
        attrs.push(format!("x{i}  TEXT NOT NULL"));
        indices.push(format!("CREATE INDEX {rel}{i}_idx ON {rel} (x{i})"));
    }

    // `it` is the iteration number, for semi-naive evaluation
    if cfg!(feature = "duckdb") {
        format!(
            r"CREATE SEQUENCE {0}_seq;
          CREATE TABLE {0} (
              id  INTEGER PRIMARY KEY DEFAULT NEXTVAL('{0}_seq'),
              it  INTEGER,
              {1}
          );
          CREATE INDEX {0}_delta_idx ON {0} (it);
          {2}{3}
         ",
            rel,
            attrs.join(",\n"),
            indices.join(";\n"),
            if indices.is_empty() { "" } else { ";" }
        )
    } else {
        format!(
            r"CREATE TABLE {0} (
              id  INTEGER PRIMARY KEY AUTOINCREMENT,
              it  INTEGER{1}
              {2}
          );
          CREATE INDEX {0}_delta_idx ON {0} (it);
          {3}{4}
         ",
            rel,
            if attrs.is_empty() { "" } else { "," },
            attrs.join(",\n"),
            indices.join(";\n"),
            if indices.is_empty() { "" } else { ";" }
        )
    }
}

fn create_tables(conn: &Connection, prog: &Mir) -> Result<()> {
    eprintln!("BEGIN;");
    conn.execute_batch("BEGIN;")?;
    for (rel, arity) in prog.arities() {
        let stmt = create_table(&rel, arity);
        eprintln!("{stmt}");
        conn.execute_batch(&stmt)?;
    }
    eprintln!("COMMIT;");
    conn.execute_batch("COMMIT;")?;
    Ok(())
}

fn exists(conn: &Connection, rel: &Rel, consts: &Vec<Const>) -> Result<bool> {
    let mut q = format!("SELECT COUNT(*) from {}", rel);
    if !consts.is_empty() {
        q += " WHERE ";
        for (i, c) in consts.iter().enumerate() {
            if i != 0 {
                q += " AND ";
            }
            q += &format!("{}.x{} = '{}'", rel, i, c);
        }
    }
    q += ";";

    eprintln!("{q}");
    let mut entries = conn.prepare_cached(&q).unwrap();
    let n: usize = entries
        .query([])?
        .next()?
        .expect("No rows for COUNT?")
        .get(0)?;
    debug_assert!(n == 0 || n == 1);
    Ok(n >= 1)
}

// TODO(lb, low): Group facts by relation, use Appender
fn insert_fact(conn: &Connection, rel: &Rel, consts: &Vec<Const>) -> Result<()> {
    let mut q = if cfg!(feature = "duckdb") {
        format!(r"INSERT INTO {0} VALUES (nextval('{0}_seq'), 0", rel)
    } else {
        let mut attrs = Vec::with_capacity(consts.len());
        attrs.push(String::from("it"));
        for i in 0..consts.len() {
            attrs.push(format!("x{i}"));
        }
        format!(r"INSERT INTO {0} ({1}) VALUES (0", rel, attrs.join(", "))
    };
    for c in consts {
        q += &format!(", '{c}'");
    }
    q += ");";

    eprintln!("{q}");
    let mut stmt = conn.prepare_cached(&q)?;
    stmt.execute([])?;
    conn.flush_prepared_statement_cache();
    Ok(())
}

fn insert_fact_if_not_exists(conn: &Connection, rel: &Rel, consts: &Vec<Const>) -> Result<()> {
    if exists(conn, rel, consts)? {
        return Ok(());
    }
    insert_fact(conn, rel, consts)
}

/// Non-recursive Datalog is equivalent to unions of conjunctive queries :-)
///
/// `it` is the current iteration number, for semi-naive evaluation.
///
/// See also https://github.com/philzook58/duckegg/blob/e6c9fc106098e837095c461521c451c18e53c091/duckegg.py#L101
fn eval_rule_query(rule: &Rule, it: usize) -> Vec<String> {
    let rel = &rule.head.rel;
    let mut rules = Vec::new();
    for delta in 0..rule.body.len() {
        // For each relation in the body, select from that relation's table
        let mut tables: Vec<String> = Vec::new();
        let mut bindings: HashMap<Term, Vec<String>> = HashMap::default();
        let mut delta_cond = String::new();
        for (i, atom) in rule.body.iter().enumerate() {
            // TODO: Make the SQL a bit clearer:
            // if i == delta {
            //     let table = format!("delta_{}{i}", atom.rel);
            // } else {
            //     let table = format!("{}{i}", atom.rel);
            // }
            let table = format!("{}{i}", atom.rel);
            tables.push(format!("{} AS {table}", atom.rel));
            for (field, term) in atom.terms.iter().enumerate() {
                bindings
                    .entry(term.clone())
                    .or_insert_with(|| Vec::with_capacity(1))
                    .push(format!("{table}.x{field}"))
            }
            // Semi-naive: only use the facts from the previous generation
            if i == delta && it > 0 {
                delta_cond = format!("{table}.it = {}", it - 1);
            }
        }

        // Project out the variables that are needed by the head
        let mut selects = Vec::new();
        for term in &rule.head.terms {
            selects.push(match term {
                Term::Const(c) => format!("'{}'", c),
                // Any of the bindings will do, they're all asserted equal in WHERE
                v @ Term::Var(_) => bindings
                    .get(v)
                    .expect("Range restriction violation!")
                    .get(0)
                    .unwrap()
                    .clone(),
            })
        }

        // Let SQL do the unification by building WHERE clauses that equate the
        // different SQL names of the same Datalog variable
        let mut unifications = Vec::new();
        for binds in bindings.values() {
            let mut iter = binds.iter();
            let first = iter.next().unwrap();
            for bind in iter {
                unifications.push(format!("{first} = {bind}"));
            }
        }
        let unification_conds = if unifications.is_empty() {
            String::from("true")
        } else {
            unifications.join(" AND ")
        };

        // Ensure the entry doesn't already exist (set semantics)
        let mut eqs = Vec::new();
        for (i, col) in selects.iter().enumerate() {
            eqs.push(format!("pre.x{i} = {col}"));
        }
        let mut not_exists = format!("SELECT * from {} AS pre", rel);
        if !eqs.is_empty() {
            not_exists += " WHERE ";
            not_exists += &eqs.join(" AND ");
        }

        // Assign each selected column of the subquery a name
        let mut selects_as = Vec::with_capacity(selects.len());
        for (i, sel) in selects.into_iter().enumerate() {
            selects_as.push(format!("{sel} AS y{i}"));
        }
        if selects_as.is_empty() {
            selects_as.push(String::from("*"));
        }

        let subquery = format!(
            "SELECT DISTINCT {} FROM {} WHERE {} AND {} AND NOT EXISTS ({})",
            selects_as.join(", "),
            tables.join(","),
            delta_cond,
            unification_conds,
            not_exists,
        );

        // Select out the necessary columns from the subquery
        let mut selected = Vec::with_capacity(selects_as.len());
        let n_selected = if selects_as[0] == "*" {
            0
        } else {
            selects_as.len()
        };
        for i in 0..n_selected {
            selected.push(format!("y{i}"));
        }

        rules.push(if cfg!(feature = "duckdb") {
            format!(
                r"INSERT INTO {0} SELECT nextval('{0}_seq'), it{1} FROM ({2});",
                rel,
                if selected.is_empty() {
                    String::from("")
                } else {
                    format!(", {}", selected.join(", "))
                },
                subquery
            )
        } else {
            let mut attrs = Vec::with_capacity(selected.len());
            for i in 0..selected.len() {
                attrs.push(format!("x{i}"));
            }
            format!(
                r"INSERT INTO {0} (it{1}) SELECT {it}{2} FROM ({3});",
                rel,
                if attrs.is_empty() {
                    String::from("")
                } else {
                    format!(", {}", attrs.join(", "))
                },
                if selected.is_empty() {
                    String::from("")
                } else {
                    format!(", {}", selected.join(", "))
                },
                subquery
            )
        });
    }
    rules
}

fn insert_facts(conn: &Connection, prog: &Mir) -> Result<()> {
    eprintln!("BEGIN;");
    conn.execute_batch("BEGIN;")?;
    conn.set_prepared_statement_cache_capacity(512); // just a guess
    for (rel, facts) in prog.facts() {
        for fact in facts {
            insert_fact_if_not_exists(conn, rel, fact)?;
        }
    }
    eprintln!("COMMIT;");
    conn.execute_batch("COMMIT;")?;
    Ok(())
}

impl Eval {
    /// Clear facts from the embedded [`Mir`] program.
    pub fn clear_facts(&mut self) {
        self.prog.clear_facts()
    }

    /// Create a new evaluator.
    ///
    /// If it makes sense for your time/space trade-off, you can call
    /// [`Eval::clear_facts`] after this.
    pub fn new(conn: Connection, prog: Mir) -> Result<Self> {
        create_tables(&conn, &prog)?;
        insert_facts(&conn, &prog)?;
        Ok(Self { conn, prog })
    }

    pub fn go(&self) -> Result<usize> {
        let mut iters = 0;
        // Execute the queries until fixpoint
        loop {
            iters += 1;
            // Build the conjunctive query for each rule
            let mut rule_queries = Vec::with_capacity(self.prog.rules().count());
            for rule in self.prog.rules() {
                rule_queries.extend(eval_rule_query(rule, iters));
            }

            let mut changed = false;
            eprintln!("BEGIN;");
            self.conn.execute_batch("BEGIN;")?;
            for q in &rule_queries {
                eprintln!("{q}");
                let n_changed = self.conn.execute(q, [])?;
                changed |= n_changed > 0;
            }
            eprintln!("END;");
            self.conn.execute_batch("END;")?;
            if !changed {
                break;
            }
        }
        Ok(iters)
    }

    /// The minimal Herbrand model (after calling [`Eval::go`]).
    pub fn model(&self) -> Result<HashMap<Rel, HashSet<Vec<Const>>>> {
        let mut m = HashMap::default();
        let mut arities = HashMap::default();
        for (rel, mut facts) in self.prog.facts() {
            m.entry(rel.clone()).or_insert_with(HashSet::default);
            arities.insert(rel.clone(), facts.next().unwrap().len());
        }
        for rule in self.prog.rules() {
            m.entry(rule.head.rel.clone())
                .or_insert_with(HashSet::default);
            arities.insert(rule.head.rel.clone(), rule.head.terms.len());
        }
        let keys = Vec::from_iter(m.keys().cloned());
        for rel in keys {
            let mut q = self
                .conn
                .prepare(&format!("SELECT DISTINCT * from {};", rel))
                .unwrap();
            let mut entries = q.query([]).unwrap();
            let arity = *arities.get(&rel).unwrap();
            while let Some(row) = entries.next()? {
                let mut fact = Vec::with_capacity(arity);
                for i in 0..arity {
                    // + 2 for id, it
                    fact.push(Const::new(row.get_unwrap(i + 2)).unwrap());
                }
                m.get_mut(&rel).unwrap().insert(fact);
            }
        }
        Ok(m)
    }

    pub fn into_connection(self) -> Connection {
        self.conn
    }

    pub fn into_program(self) -> Mir {
        self.prog
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{Ast, Atom, Const, Rel, Rule, Term};
    use crate::mir::Mir;

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
    fn test_nullary_init() {
        let prog = Mir::new(Ast::new(vec![null_fact()]).unwrap()).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        let eval = Eval::new(conn, prog).unwrap();
        assert_eq!(1, eval.go().unwrap());
        let _m = eval.model().unwrap();
    }

    #[test]
    fn test_unary_init() {
        let prog = Mir::new(Ast::new(vec![unary_fact()]).unwrap()).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        let eval = Eval::new(conn, prog).unwrap();
        assert_eq!(1, eval.go().unwrap());
        let _m = eval.model().unwrap();
    }

    #[test]
    fn test_same_fact() {
        let prog = Mir::new(Ast::new(vec![null_fact(), null_fact()]).unwrap()).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        let eval = Eval::new(conn, prog).unwrap();
        let conn = eval.into_connection();
        let mut entries = conn.prepare("SELECT COUNT(*) from r;").unwrap();
        let n: usize = entries
            .query([])
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .get_unwrap(0);
        assert_eq!(1, n);
    }

    #[test]
    fn test_nullary_copy() {
        // prog:
        //
        //   r.
        //   s :- r.
        //
        let prog = Mir::new(
            Ast::new(vec![
                null_fact(),
                Rule::new(
                    Atom::new(Rel::new(String::from("s")), Vec::new()),
                    vec![null_atom()],
                ),
            ])
            .unwrap(),
        )
        .unwrap();
        let conn = Connection::open_in_memory().unwrap();
        let eval = Eval::new(conn, prog).unwrap();
        assert_eq!(2, eval.go().unwrap());
        let _m = eval.model().unwrap();
    }
}
