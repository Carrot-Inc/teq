// An export through an extension's using-clause path of a member whose parameters have defaults: scalac 3.8.4's
// forwarder keeps no default (`(b: Int, c: Int): Int`), so a call that leaves both out is rejected. dotty reports the
// first missing parameter alone: `tryDefault`'s `missingArg(n)` fails the application without matching the rest
// (Applications.scala:846-847, 893-894). teq reports two errors at 15:23, "missing argument for parameter b" and
// "... parameter c", and prints the parameters `Int @RuntimeChecked` (C1's printer). Found while narrowing
// tests/modules/exportinlinedefaults.
class Ops(val s: String):
  inline def r(a: Int)(inline b: Int = 1, inline c: Int = a + 2): Int = a * 100 + b * 10 + c + s.length * 1000
object WU:
  extension (s: String)(using n: Int)
    def ops: Ops = Ops(s * n)
    export ops.r
@main def run(): Unit =
  given Int = 2
  println(WU.r("x")(4)())
