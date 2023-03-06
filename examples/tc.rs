//! Transitive closure

use duckalog::{
    ast::{Ast, Atom, Const, Rel, Rule, Term, Var},
    eval::Eval,
    mir::Mir,
};
use duckdb::Connection;

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
            Atom::new(path, vec![x.clone(), z.clone()]),
            vec![
                Atom::new(edge.clone(), vec![x.clone(), y.clone()]),
                Atom::new(edge.clone(), vec![y.clone(), z.clone()]),
            ],
        ),
    ])
    .unwrap();

    let mut mir = Mir::new(ast).unwrap();
    let a = Const::new("a".to_string()).unwrap();
    let b = Const::new("b".to_string()).unwrap();
    let c = Const::new("c".to_string()).unwrap();
    mir.add_fact(&edge, vec![a, b.clone()]);
    mir.add_fact(&edge, vec![b, c]);

    let conn = Connection::open_in_memory().unwrap();
    let exec = Eval::new(conn, mir).unwrap();
    assert_eq!(3, exec.go().unwrap());
}
