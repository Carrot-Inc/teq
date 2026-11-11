// A downstream of a/, whose `(x: Paths.this.Q)` qualifies the object's own `this`. scalac 3.8.4 writes it
// `TYPED(TERMREFdirect x, SELECTtpt Q (QUALTHIS (IDENTtpt Paths)))`; every teq build that pickled a/ panicked on it,
// "resolved selection" (pickle/term.rs:950): the kept type tree's path took `Paths.this` for no enclosing class, as it
// matched a class frame's name alone, where dotty's `qualifyingClass.qualifies` (TypeAssigner.scala:24-32) also takes
// a module class whose name without the module suffix is the qualifier. Found by the interim reviewer of the workers
// exports helper's push 9, and by Codex's review of its pushes 9 and 10.
package tqb

@main def run(): Unit =
  println(tq.Paths.a(3))
