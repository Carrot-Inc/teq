// A case object and an enum's singleton case are their own mirrors (`Synthesizer`'s `singletonPath`,
// `SyntheticMembers.addMirrorSupport`'s `Mirror.Singleton` parent), whose `fromProduct` answers the object.
import scala.deriving.Mirror
case object O
enum E:
  case A
@main def run(): Unit =
  println(summon[Mirror.Of[O.type]] eq O)
  println(summon[Mirror.Of[E.A.type]] eq E.A)
  println(summon[Mirror.ProductOf[O.type]].fromProduct(EmptyTuple) eq O)
  println(summon[Mirror.Of[E.A.type]].isInstanceOf[Mirror.Singleton])
  println(summon[Mirror.ProductOf[E.A.type]].fromProduct(EmptyTuple) eq E.A)
