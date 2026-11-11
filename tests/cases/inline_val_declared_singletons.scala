// An inline val's declared type may be the singleton of a constant (`Int.MaxValue.type`, a final val's
// `t.type`): its type widened to the constant (`InlineVals`' `widenTermRefExpr`). scalac prints the lines of the
// .expected file.
object C:
  final val t = true
  inline val b: Int.MaxValue.type = Int.MaxValue
  inline val c: t.type = t
@main def run(): Unit = println((C.b, C.c))
