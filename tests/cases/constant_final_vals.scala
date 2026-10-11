// A `final val` of a constant type holds no field (dotty's `isConstExprFinalVal`, `Memoize`): a
// read answers the constant, behind a receiver whose effects run first too, and an initialiser
// that is no literal runs once where the object, the class or the trait's class is made.
object O:
  final val K = 7
  final val W: Int = 8
  final val E: 9 = { println("init O"); 9 }
  def touch: Int = 0
trait T:
  final val TK = 10
  final val TE: 11 = { println("init T"); 11 }
class C extends T:
  final val S = "s"
def made(): C = { println("made"); new C }

@main def run(): Unit =
  println(O.K)
  println(O.E)
  println(O.touch)
  println(O.W)
  val c = new C
  println(c.TK + " " + c.S)
  println(made().TK)
  println(made().S)
  val t: T = c
  println(t.TE)
