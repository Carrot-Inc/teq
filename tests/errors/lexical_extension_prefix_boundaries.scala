// expect: 24:45: error: no given instance of type Second was found
// expect: 25:30: error: no given instance of type Need was found
// expect: lexical_extension_prefix_boundaries.scala:26:
// expect: 3 errors found
// The prefix `f(qual)` ends where the call's next application begins: an explicit `using` list
// is that application, and type arguments written for the method's own type parameters leave its
// clauses unapplied. A using clause past either is the selected lexical extension's, its missing
// given reported, not a reason to take the companion's (scalac: "No given instance of type Second"
// and "No given instance of type Need", for an explicit first `using` list, type arguments
// alone, and type arguments before a value argument).
trait First
trait Second
trait Need
class R
object R:
  extension (r: R) def pick(using First)(x: Int): String = "companion"
  extension (r: R) def poly[A]: String = "companion"
  extension (r: R) def polyx[A](x: Int): String = "companion"
object Main:
  extension (r: R) def pick(using First)(using Second)(x: Int): String = "lexical"
  extension (r: R) def poly[A](using Need): String = "lexical"
  extension (r: R) def polyx[A](using Need)(x: Int): String = "lexical"
  def main(args: Array[String]): Unit =
    println((new R).pick(using new First {})(1))
    println((new R).poly[Int])
    println((new R).polyx[Int](1))
