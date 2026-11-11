// expect: 19:26: error: ambiguous given instances for TC: a, b
// expect: 1 error found
// `a` and `b` are ranked alike; `c`, better than both, could heal the ambiguity (`healAmbiguous`, Implicits.scala
// 1472-1490), but its own `Need` is ambiguous, so nothing heals it and the search fails with the original ambiguity
// (`case _ => fail`), not the healer's: scalac 3.8.4 reports "Ambiguous given instances: both given instance a in trait
// Low and given instance b in trait Low match type TC". The class context holds three names, whose order dotty's
// `memberNames` settles.
trait TC
trait Need
object Needs:
  given n1: Need = new Need {}
  given n2: Need = new Need {}
trait Low:
  given a: TC = new TC {}
  given b: TC = new TC {}
object O extends Low:
  import Needs.given
  given c(using Need): TC = new TC {}
  def chosen = summon[TC]
@main def run(): Unit = println(O.chosen)
