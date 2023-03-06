use duckdb::{Connection, Result};

use crate::ast::{GroundAtom, Program, Rel};

#[derive(Debug)]
pub struct Eval {
    conn: Connection,
    prog: Program,
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

fn create_tables(conn: &Connection, prog: &Program) -> Result<()> {
    conn.execute_batch("BEGIN;")?;
    for (rel, arity) in prog.arities() {
        let stmt = create_table(&rel, arity);
        conn.execute_batch(&stmt)?;
    }
    conn.execute_batch("COMMIT;")?;
    Ok(())
}

fn exists(conn: &Connection, fact: &GroundAtom) -> Result<bool> {
    let mut q = format!("SELECT COUNT(*) from {}", fact.rel);
    if !fact.terms.is_empty() {
        q += " WHERE ";
        for (i, term) in fact.terms.iter().enumerate() {
            q += &format!("{}.x{} = '{}'", fact.rel, i, term);
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
fn insert_fact(conn: &Connection, fact: &GroundAtom) -> Result<()> {
    let mut q = format!(r"INSERT INTO {0} VALUES (nextval('{0}_seq')", fact.rel);
    for _ in &fact.terms {
        q += ", ?";
    }
    q += ");";

    let mut stmt = conn.prepare_cached(&q)?;
    stmt.execute(duckdb::params_from_iter(&fact.terms))?;
    conn.flush_prepared_statement_cache();
    Ok(())
}

fn insert_fact_if_not_exists(conn: &Connection, fact: &GroundAtom) -> Result<()> {
    if exists(conn, fact)? {
        return Ok(());
    }
    insert_fact(conn, fact)
}

fn insert_facts(conn: &Connection, prog: &Program) -> Result<()> {
    conn.execute_batch("BEGIN;")?;
    conn.set_prepared_statement_cache_capacity(512); // just a guess
    for rule in &prog.rules {
        if rule.is_fact() {
            let ground = rule
                .head
                .clone()
                .ground()
                .expect("Range restriction violation!");
            insert_fact_if_not_exists(conn, &ground)?;
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

    pub fn into_connection(self) -> Connection {
        self.conn
    }

    pub fn into_program(self) -> Program {
        self.prog
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{Atom, Const, Rel, Rule, Term};

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

    #[test]
    fn test_same_fact() {
        let prog = Program::new(vec![null_fact(), null_fact()]).unwrap();
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
