use duckdb::{Connection, Result};

use crate::ast::{Const, Rel, Rule};
use crate::mir::Mir;

#[derive(Debug)]
pub struct Eval {
    conn: Connection,
    prog: Mir,
}

fn create_table(rel: &Rel, arity: usize) -> String {
    let mut attrs = String::new();
    for i in 0..arity {
        attrs += &format!("x{i}  TEXT NOT NULL");
        if i + 1 != arity {
            attrs += ",\n";
        }
    }
    // TODO(lb, high): add delta for semi-naive
    format!(
        r"CREATE SEQUENCE {0}_seq;
          CREATE TABLE {0} (
              id  INTEGER PRIMARY KEY DEFAULT NEXTVAL('{0}_seq'),
              {1}
          );
         ",
        rel, attrs
    )
}

fn create_tables(conn: &Connection, prog: &Mir) -> Result<()> {
    conn.execute_batch("BEGIN;")?;
    for (rel, arity) in prog.arities() {
        let stmt = create_table(&rel, arity);
        conn.execute_batch(&stmt)?;
    }
    conn.execute_batch("COMMIT;")?;
    Ok(())
}

fn exists(conn: &Connection, rel: &Rel, consts: &Vec<Const>) -> Result<bool> {
    let mut q = format!("SELECT COUNT(*) from {}", rel);
    if !consts.is_empty() {
        q += " WHERE ";
        for (i, c) in consts.iter().enumerate() {
            q += &format!("{}.x{} = '{}'", rel, i, c);
        }
    }
    q += ";";

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
    let mut q = format!(r"INSERT INTO {0} VALUES (nextval('{0}_seq')", rel);
    for _ in consts {
        q += ", ?";
    }
    q += ");";

    let mut stmt = conn.prepare_cached(&q)?;
    stmt.execute(duckdb::params_from_iter(consts))?;
    conn.flush_prepared_statement_cache();
    Ok(())
}

fn insert_fact_if_not_exists(conn: &Connection, rel: &Rel, consts: &Vec<Const>) -> Result<()> {
    if exists(conn, rel, consts)? {
        return Ok(());
    }
    insert_fact(conn, rel, consts)
}

/// See also https://github.com/philzook58/duckegg/blob/e6c9fc106098e837095c461521c451c18e53c091/duckegg.py#L101
fn eval_rule(_conn: &Connection, _rule: &Rule) -> Result<bool> {
    // TODO
    Ok(false)
}

fn insert_facts(conn: &Connection, prog: &Mir) -> Result<()> {
    conn.execute_batch("BEGIN;")?;
    conn.set_prepared_statement_cache_capacity(512); // just a guess
    for (rel, facts) in prog.facts() {
        for fact in facts {
            insert_fact_if_not_exists(conn, rel, fact)?;
        }
    }
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
        loop {
            iters += 1;
            let mut changed = false;
            for rule in self.prog.rules() {
                changed |= eval_rule(&self.conn, rule)?;
            }
            if !changed {
                break;
            }
        }
        Ok(iters)
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
        Eval::new(conn, prog).unwrap();
    }

    #[test]
    fn test_unary_init() {
        let prog = Mir::new(Ast::new(vec![unary_fact()]).unwrap()).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        Eval::new(conn, prog).unwrap();
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
}
