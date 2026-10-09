// expect: 8:19: error: pattern's type (Int, Some[Int]) is more specialized than the right hand side expression's type (Int, Option[Int])
// absent: 8:19: warning: the type test for (Int, Some[Int]) cannot be checked at runtime
// A generator's function typed ahead of the choice among `map`'s alternatives
// (`Applications.pretypeArgs`) keeps its pattern checked as irrefutable (`Checking.checkIrrefutable`),
// reported at the pattern's type; scalac stops at that error before it warns of the type test.
@main def run(): Unit =
  val m: Map[Int, Option[Int]] = Map(1 -> Some(2))
  val a = for (p: (Int, Some[Int])) <- m yield p._1
  println(a)
