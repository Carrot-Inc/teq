package names

// The classes stored inline bodies make, named as the retype path names them: the same factory
// expanded twice under one outer site (`two`), an expansion in a branch the body discards then a
// live one at the same site (`chosen`, an `inline if`; `plain`, an `if` over a constant
// argument), a lambda's class of a trait with one abstract method twice (`steps`), a class whose
// captures the typing meets out of evaluation order (`capturing`: a lambda argument typed after
// the argument beside it, as the retype path's inference types it; `passing`: a reference an
// inner expansion brings), an anonymous class made in another's parent's arguments (`holding`),
// in four files
// the workers split between them. tests/workers.sh compares the expansion by substitution's files
// with the retype path's at one, two and sixteen workers; tests/split-watch.sh a session's after
// edits before and after the sites with fresh builds of both.
@main def run(): Unit =
  println(List(F0.shows, F1.shows, F2.shows, F3.shows).flatten.mkString(" "))
