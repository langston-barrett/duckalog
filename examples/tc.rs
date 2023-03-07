//! Transitive closure

#[cfg(feature = "duckdb")]
use duckdb::Connection;
#[cfg(feature = "sqlite")]
use rusqlite::Connection;

use duckalog::{
    ast::{Ast, Atom, Const, Rel, Rule, Term, Var},
    eval::Eval,
    mir::Mir,
};

fn main() {
    let x = Term::Var(Var::new("X".to_string()).unwrap());
    let y = Term::Var(Var::new("Y".to_string()).unwrap());
    let z = Term::Var(Var::new("Z".to_string()).unwrap());
    let edge = Rel::new("edge".to_string());
    let path = Rel::new("path".to_string());
    let ast = Ast::new(vec![
        Rule::new(
            Atom::new(path.clone(), vec![x.clone(), y.clone()]),
            vec![Atom::new(edge.clone(), vec![x.clone(), y.clone()])],
        ),
        Rule::new(
            Atom::new(path.clone(), vec![x.clone(), z.clone()]),
            vec![
                Atom::new(edge.clone(), vec![x.clone(), y.clone()]),
                Atom::new(path.clone(), vec![y.clone(), z.clone()]),
            ],
        ),
    ])
    .unwrap();

    let mut mir = Mir::new(ast).unwrap();
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(std::io::stdin());
    for rec0 in reader.records() {
        let rec = rec0.unwrap();
        if rec.len() != 2 {
            eprintln!("Bad record");
            continue;
        }
        mir.add_fact(
            &edge,
            vec![
                Const::new(rec[0].to_string()).unwrap(),
                Const::new(rec[1].to_string()).unwrap(),
            ],
        );
    }

    let conn = Connection::open_in_memory().unwrap();
    let exec = Eval::new(conn, mir).unwrap();
    exec.go().unwrap();
    let model = exec.model().unwrap();
    let edges = model.get(&edge).unwrap();
    let paths = model.get(&path).unwrap();
    debug_assert!(paths.len() >= edges.len());
    let lock = std::io::stdout();
    let mut writer = csv::WriterBuilder::new()
        .quote_style(csv::QuoteStyle::NonNumeric)
        .from_writer(lock);
    for path in paths {
        debug_assert!(path.len() == 2);
        writer
            .write_record(&[String::from(path[0].clone()), String::from(path[1].clone())])
            .unwrap();
    }
}
