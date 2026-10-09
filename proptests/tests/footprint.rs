//! The two properties the footprint of the crate is measured with: one that starts nothing and
//! one that starts a process per case.

use hegel::generators as gs;
use hegel::TestCase;
use std::process::Command;

#[hegel::test]
fn addition_commutes(tc: TestCase) {
    let a = tc.draw(gs::integers::<i32>());
    let b = tc.draw(gs::integers::<i32>());
    assert_eq!(a.wrapping_add(b), b.wrapping_add(a));
}

#[hegel::test]
fn a_build_per_case(tc: TestCase) {
    let n = tc.draw(gs::integers::<i32>());
    let dir = teq_proptests::work_root().join("footprint");
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("src/main.scala"),
        format!("@main def run(): Unit = println({n})\n"),
    )
    .unwrap();
    let out = Command::new(teq_proptests::teq())
        .args(["compiler", "build"])
        .arg(dir.join("src"))
        .arg("--split")
        .arg(dir.join("out"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}
