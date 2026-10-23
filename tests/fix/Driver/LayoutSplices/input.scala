/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// A splice's block is from its `{` to its `}`, as scalameta's, though its expression is in parentheses or after a
// comment; a bare splice `$x` is its name alone.
object Splices:
  val marker = 0 // assert: DumpLayout.layout
  val s = s"${ (1) }"
  val s2 = s"${ /* comment */ 1 }"
  val s3 = s"${s"${marker}"} $marker"
