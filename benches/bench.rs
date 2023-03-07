use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};

#[cfg(feature = "duckdb")]
use duckdb::Connection;
#[cfg(feature = "sqlite")]
use rusqlite::Connection;

use duckalog::ast::{Ast, Atom, Const, Rel, Rule, Term, Var};
use duckalog::eval::Eval;
use duckalog::mir::Mir;

fn eval(mir: Mir) -> HashMap<Rel, HashSet<Vec<Const>>> {
    let conn = Connection::open_in_memory().unwrap();
    let exec = Eval::new(conn, mir).unwrap();
    exec.go().unwrap();
    exec.model().unwrap()
}

/// ```
/// path(X, Y) :- edge(X, Y).
/// path(X, Z) :- path(X, Y), edge(Y, Z).
/// ```
fn tc() -> (Mir, Rel, Rel) {
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
                Atom::new(path.clone(), vec![x.clone(), y.clone()]),
                Atom::new(edge.clone(), vec![y.clone(), z.clone()]),
            ],
        ),
    ])
    .unwrap();
    let mir = Mir::new(ast).unwrap();
    (mir, edge, path)
}

/// Transitive closure on a long line
pub fn tc_line(c: &mut Criterion, n: usize) {
    let (mut mir, edge, path) = tc();
    for i in 0..n {
        let ci = Const::new(format!("c{i}")).unwrap();
        let ci1 = Const::new(format!("c{}", i + 1)).unwrap();
        mir.add_fact(&edge, vec![ci, ci1]);
    }
    c.bench_function(&format!("tc_line_{n}"), |b| {
        b.iter(|| {
            let model = eval(mir.clone());
            let paths = model.get(black_box(&path)).unwrap();
            assert_eq!(paths.len(), n * (n + 1) / 2)
        })
    });
}

pub fn tc_line_10(c: &mut Criterion) {
    tc_line(c, 10);
}

pub fn tc_line_20(c: &mut Criterion) {
    tc_line(c, 20);
}

pub fn tc_line_30(c: &mut Criterion) {
    tc_line(c, 30);
}

/// Transitive closure on a complete graph
pub fn tc_complete(c: &mut Criterion, n: usize) {
    let (mut mir, edge, path) = tc();
    for i in 0..n {
        for j in 0..n {
            let ci = Const::new(format!("c{i}")).unwrap();
            let cj = Const::new(format!("c{j}")).unwrap();
            mir.add_fact(&edge, vec![ci, cj]);
        }
    }
    c.bench_function(&format!("tc_complete_{n}"), |b| {
        b.iter(|| {
            let model = eval(mir.clone());
            let paths = model.get(black_box(&path)).unwrap();
            assert_eq!(paths.len(), n.pow(2))
        })
    });
}

pub fn tc_complete_10(c: &mut Criterion) {
    tc_complete(c, 10);
}

pub fn tc_complete_20(c: &mut Criterion) {
    tc_complete(c, 20);
}

pub fn tc_complete_30(c: &mut Criterion) {
    tc_complete(c, 30);
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets =
      tc_line_10, tc_line_20, tc_line_30,
      tc_complete_10, tc_complete_20, tc_complete_30,
}
criterion_main!(benches);
