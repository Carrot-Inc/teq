// The probe's one deprecated definition is the std's, scala-library's `Predef.copyArrayToImmutableIndexedSeq`, named
// through `Predef` and bare: the lean std reads its mark without turning the reference checks on, and a reference
// resolving it turns them on before it is checked.
@main def run(): Unit =
  val a = Array(1, 2, 3)
  println(Predef.copyArrayToImmutableIndexedSeq(a).length)
  println(copyArrayToImmutableIndexedSeq(a).sum)
