// expect: 19:49: error: ambiguous extension methods: both a and b provide pick on R
// expect: 1 error found
// Two givens that tie, their extensions' receivers alike, are an ambiguity a later given heals
// only where it is strictly better than both by the givens themselves (dotty's `healAmbiguous`),
// not by a narrower receiver of its extension (scalac: "both object a in object Main and object b
// in object Main provide an extension method `pick` on R"; an earlier teq took `A`): `c`, whose
// receiver is narrower, is no better a given than `a` or `b`.
class R
trait A:
  extension (r: Any) def pick: String = "A"
trait B:
  extension (r: Any) def pick: String = "B"
trait C:
  extension (r: R) def pick: String = "C"
object Main:
  given a: A = new A {}
  given b: B = new B {}
  given c: C = new C {}
  def main(args: Array[String]): Unit = println((new R).pick)
