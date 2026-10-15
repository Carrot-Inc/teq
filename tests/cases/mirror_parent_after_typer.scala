// A companion is its class's mirror from `SyntheticMembers.addMirrorSupport`, after typing
// (`addParent`'s `installAfter`): no type of the program's sees it extend `Mirror.Product`, a
// summon casts it (`Synthesizer`'s `mirrorRef.cast`), and at run time it is one.
import scala.deriving.Mirror
case class P(x: Int)
sealed trait S
object S
case class A(n: Int) extends S
case object B extends S
@main def run(): Unit =
  println((P: Any).isInstanceOf[Mirror.Product])
  val made = (P: Any) match
    case m: Mirror.Product => m.fromProduct(Tuple1(1))
    case _ => "none"
  println(made)
  val pm = summon[Mirror.ProductOf[P]]
  println(pm.fromProduct(Tuple1(2)))
  println(pm eq P)
  val sm = summon[Mirror.SumOf[S]]
  println(sm.ordinal(B))
  println(sm eq S)
  println((S: Any).isInstanceOf[Mirror.Sum])
  println((B: Any).isInstanceOf[Mirror.Singleton])
