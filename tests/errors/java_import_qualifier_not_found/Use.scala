// An import whose qualifier names nothing is refused at its last selector in the reference's words (E008), a static
// one whose member is unused too; the name the import claims is bound by no later scope (the package's own Node is
// not taken) and is told nothing more where it is used.
// expect: J.java:2:13: error: value nope is not a member of <root>
// expect: J.java:3:22: error: value Nada is not a member of p
object Use { def f = new p.J }
