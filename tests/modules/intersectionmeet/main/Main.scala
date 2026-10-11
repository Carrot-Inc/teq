package imm

import ima.*

// The member of an intersection of the upstream products' traits is merged where it is looked
// up (Denotations.meet, infoMeet): expanded to a function or generic, its result the meet. In
// `main`, which no reader of pickles checks: scalac's own pickle of such a selection fails
// -Ycheck after readTasty too.
def use(x: A & B): Int => (P & Q) = x.value
def generic(x: A & B): P & Q = x.poly[String]("s")

@main def run(): Unit =
  println(use(new C)(1).q)
  println(generic(new C).p)
