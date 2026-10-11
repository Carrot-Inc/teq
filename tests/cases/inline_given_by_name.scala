// An inline given selected by its name is expanded there, as dotty's `Desugar` makes `inline given g: T = e` an
// inline method: an object's, a trait's through an instance, an imported one, each use running the right-hand side
// again (master: `NoSuchMethodError` on the JVM, `undefined` on JavaScript, `null` in the interpreter, no method
// being written for it).
var count = 0
object O:
  inline given g: Int = { count += 1; count * 10 }
  inline given h(using s: String): Int = s.length
trait T { inline given tg: Int = 43 }
class C extends T
trait Show[A] { def show(a: A): String }
object S:
  transparent inline given showInt: Show[Int] = new Show[Int] { def show(a: Int) = s"i$a" }
@main def run() =
  println(O.g)
  println(O.g)
  println(summon[Int](using O.g))
  println(count)
  println(O.h(using "abc"))
  println(new C().tg)
  println(S.showInt.show(1))
  import O.g
  println(g)
