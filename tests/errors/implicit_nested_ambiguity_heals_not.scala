// expect: 21:30: error: ambiguous given instances for Need: n1, n2
// expect: 1 error found
// `implicit_migration_negation` without the migration setting: `a`'s ambiguous `Need` is healed only by a candidate
// strictly better than `a` (`healAmbiguous`, Implicits.scala:1472-1490), which `b` is not, so the search fails with
// that ambiguity: scalac 3.8.4 reports "No best given instance of type TC was found ... But both value n1 in object O
// and value n2 in object O match type Need".
trait TC
trait Need
trait X
trait Y
object XY {
  implicit val x: X = new X {}
  implicit val y: Y = new Y {}
}
object O {
  import XY._
  implicit val n1: Need = new Need {}
  implicit val n2: Need = new Need {}
  implicit def a(implicit n: Need): TC = new TC {}
  implicit def b(implicit x: X, y: Y): TC = new TC {}
  def chosen = implicitly[TC]
}
object Main {
  def main(args: Array[String]): Unit = println(O.chosen != null)
}
