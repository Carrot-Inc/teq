// A case class needs a leading non-implicit parameter list (E144, scalac's Desugar): a using or implicit first clause
// is told at the class's name, an enum's class case's too; a written empty clause before it, or a context bound's
// evidence, is none.
// expect: 11:12: error: A case class must have at least one leading non-implicit parameter list
// expect: 12:12: error: A case class must have at least one leading non-implicit parameter list
// expect: 13:23: error: A case class must have at least one leading non-implicit parameter list
// expect: 17:8: error: A case class must have at least one leading non-implicit parameter list
class A
class B
given A = A()
case class K(using val a: A, val b: B)
case class L(implicit val a: A)
object O { case class M(using a: A)(x: Int) }
case class Empty()(using a: A)
case class Bound[T: Ordering](x: T)
enum E:
  case K(using a: A)
