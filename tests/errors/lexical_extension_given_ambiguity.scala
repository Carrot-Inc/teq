// expect: 25:13: error: ambiguous extension methods: both a and b provide pick on R
// expect: 26:13: error: ambiguous extension methods: both c and d provide pick on S
// expect: 2 errors found
// Two givens of one rank that both provide the extension are an ambiguity, in the implicit
// scope (two equal companion givens: scalac "both
// object a in object R and object b in object R provide an extension method `pick` on R") as
// among the givens in scope; the first is not taken for being first.
class R
class S
trait Ops:
  extension (r: R) def pick: String
trait SOps:
  extension (s: S) def pick: String
object R:
  given a: Ops with
    extension (r: R) def pick: String = "a"
  given b: Ops with
    extension (r: R) def pick: String = "b"
object Main:
  given c: SOps with
    extension (s: S) def pick: String = "c"
  given d: SOps with
    extension (s: S) def pick: String = "d"
  def main(args: Array[String]): Unit =
    println((new R).pick)
    println((new S).pick)
