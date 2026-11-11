package tsb

import tsa.Lib

// Over a/'s products the cast before `_1` stays: a Tuple3 the downstream casts to the chain, an erased Product,
// reaches `head` and fails there, as in scalac 3.8.4's builds. scalac-lean-b reads the chain's alias in teq's lean
// pickles as scala-library's, `scala.Tuple$package.EmptyTuple` (leanaliasowner).
@main def run(): Unit =
  println(Lib.useHead())
  try println(Lib.head(((2, 3, 4): Any).asInstanceOf[Int *: Int *: EmptyTuple]))
  catch case _: ClassCastException => println("CCE")
