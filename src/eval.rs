// use duckdb::arrow::record_batch::RecordBatch;
// use duckdb::arrow::util::pretty::print_batches;
// use duckdb::{params, Connection, Result};
use duckdb::{Connection, Result};

use crate::ast::Program;

#[derive(Debug)]
pub struct Eval {
    // TODO: rm pub
    pub conn: Connection,
    pub prog: Program,
}

impl Eval {
    pub fn new(conn: Connection, prog: Program) -> Result<Self> {
        for (rel, arity) in prog.arities() {
            let mut attrs = String::new();
            for i in 0..arity {
                attrs += &format!("x{i}  TEXT NOT NULL");
                if i + 1 != arity {
                    attrs += ",\n";
                }
            }
            // TODO: delta
            let stmt = format!(
                r"CREATE SEQUENCE {0}_seq;
              CREATE TABLE {0} (
                  id  INTEGER PRIMARY KEY DEFAULT NEXTVAL('seq'),
                  {1}
              );
             ",
                rel, attrs
            );
            eprintln!("{}", stmt);
            conn.execute_batch(&stmt)?;
        }
        Ok(Self { conn, prog })
    }
}

// fn main() -> Result<()> {
//     let conn = Connection::open_in_memory()?;

//     conn.execute_batch(
//         r"CREATE SEQUENCE seq;
//           CREATE TABLE person (
//                   id              INTEGER PRIMARY KEY DEFAULT NEXTVAL('seq'),
//                   name            TEXT NOT NULL,
//                   data            BLOB
//                   );
//          ",
//     )?;
//     let me = Person {
//         id: 0,
//         name: "Steven".to_string(),
//         data: None,
//     };
//     conn.execute(
//         "INSERT INTO person (name, data) VALUES (?, ?)",
//         params![me.name, me.data],
//     )?;

//     let mut stmt = conn.prepare("SELECT id, name, data FROM person")?;
//     let person_iter = stmt.query_map([], |row| {
//         Ok(Person {
//             id: row.get(0)?,
//             name: row.get(1)?,
//             data: row.get(2)?,
//         })
//     })?;

//     for person in person_iter {
//         println!("Found person {:?}", person.unwrap());
//     }

//     // query table by arrow
//     let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
//     print_batches(&rbs);
//     Ok(())
// }

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
