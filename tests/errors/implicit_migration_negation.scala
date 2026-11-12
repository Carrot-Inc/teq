// teq: --source 3.0-migration --werror
// expect: 22:30: migration warning: Ambiguous implicits value n1 in object O and value n2 in object O
// Under `-source 3.0-migration` a candidate whose own search is ambiguous (`a`'s `Need`) is a plain failure, the
// search going on from what it found (`rank`'s `migrateTo3` arm, Implicits.scala:1497-1506), and where it then succeeds
// (`b`) `warnAmbiguousNegation` warns (1546-1554): scalac 3.8.4 compiles the program with the migration warning
// "Ambiguous implicits value n1 in object O and value n2 in object O / seem to be used to implement a local failure in
// order to negate an implicit search. ...". The object holds four implicit names. `--werror` is to fail the run.
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
