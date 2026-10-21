// `::` applied as an extractor in a pattern, as EmptyCollectionsUnified's fixture writes it: scalac accepts it, teq
// reports "expected a pattern, found identifier" (tests/fix/REFERENCES.md).
@main def run(): Unit =
  val a = List(1)
  println(a match { case ::(_, Nil) => 1; case _ => 2 })
