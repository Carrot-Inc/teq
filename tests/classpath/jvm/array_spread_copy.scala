// jars: scala-library
// The control of tests/cases/array_spread_alias: a conversion the program writes copies as scalac's does, the
// array it is given marked spread by nothing (`Predef.copyArrayToImmutableIndexedSeq`, which JavaScript's std lacks),
// while the array spread itself is wrapped as it is.
def keep(xs: String*): Seq[String] = xs
@annotation.nowarn("cat=deprecation") @main def run() =
  val c = Array("c")
  val copied = keep(Predef.copyArrayToImmutableIndexedSeq(c)*)
  val wrapped = keep(c*)
  c(0) = "changed"
  println(copied.head)
  println(wrapped.head)
