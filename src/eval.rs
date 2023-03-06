use duckdb::{Connection, Result};

use crate::ast::{Atom, Program, Rel};

#[derive(Debug)]
pub struct Eval {
    // TODO: rm pub
    pub conn: Connection,
    pub prog: Program,
}

fn create_table(rel: &Rel, arity: usize) -> String {
    let mut attrs = String::new();
    for i in 0..arity {
        attrs += &format!("x{i}  TEXT NOT NULL");
        if i + 1 != arity {
            attrs += ",\n";
        }
    }
    // TODO: delta
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

fn create_tables(conn: &Connection, prog: &Program) -> Result<()> {
    conn.execute_batch("BEGIN;")?;
    for (rel, arity) in prog.arities() {
        let stmt = create_table(&rel, arity);
        conn.execute_batch(&stmt)?;
    }
    conn.execute_batch("COMMIT;")?;
    Ok(())
}

fn insert_fact(conn: &Connection, fact: &Atom) -> Result<()> {
    let mut q = format!(r"INSERT INTO {0} VALUES (nextval('{0}_seq')", fact.rel);
    for _ in &fact.terms {
        q += ", ?";
    }
    q += ");";
    let mut stmt = conn.prepare_cached(&q)?;
    let mut values = Vec::with_capacity(fact.terms.len());
    for t in &fact.terms {
        match t {
            crate::Term::Var(_) => panic!("Range restriction violation!"),
            crate::Term::Const(c) => {
                values.push(c);
            }
        }
    }
    stmt.execute(duckdb::params_from_iter(values))?;
    conn.flush_prepared_statement_cache();
    Ok(())
}

fn insert_facts(conn: &Connection, prog: &Program) -> Result<()> {
    conn.execute_batch("BEGIN;")?;
    conn.set_prepared_statement_cache_capacity(512); // just a guess
    for rule in &prog.rules {
        if rule.is_fact() {
            insert_fact(conn, &rule.head)?;
        }
    }
    conn.execute_batch("COMMIT;")?;
    Ok(())
}

impl Eval {
    pub fn new(conn: Connection, prog: Program) -> Result<Self> {
        create_tables(&conn, &prog)?;
        insert_facts(&conn, &prog)?;
        Ok(Self { conn, prog })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Atom, Const, Rel, Rule, Term};

    fn null_atom() -> Atom {
        Atom::new(Rel::new(String::from("r")), Vec::new())
    }

    fn null_fact() -> Rule {
        Rule::new(null_atom(), Vec::new())
    }

    fn unary_atom() -> Atom {
        Atom::new(
            Rel::new(String::from("r")),
            vec![Term::Const(Const::new(String::from("c")))],
        )
    }

    fn unary_fact() -> Rule {
        Rule::new(unary_atom(), Vec::new())
    }

    #[test]
    fn test_nullary_init() {
        let prog = Program::new(vec![null_fact()]).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        Eval::new(conn, prog).unwrap();
    }

    #[test]
    fn test_unary_init() {
        let prog = Program::new(vec![unary_fact()]).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        Eval::new(conn, prog).unwrap();
    }
}
