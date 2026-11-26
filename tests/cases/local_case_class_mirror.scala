// A case class local to a block takes its companion as its Mirror.Product, as scalac's does (dotty's
// `useCompanionAsProductMirror`): two summons are the same object and its fromProduct constructs, in a def's body, for
// a generic class, in a lambda and in a class's method. The companion is the mirror its type names, so a build from
// the program's TASTy finds it one.
import scala.deriving.Mirror

class K:
  def m: String =
    case class L(a: Int, b: String)
    val mirror = summon[Mirror.ProductOf[L]]
    (mirror eq summon[Mirror.ProductOf[L]], mirror.fromProduct((1, "k"))).toString

@main def run(): Unit =
  case class L(a: Int)
  val m1 = summon[Mirror.ProductOf[L]]
  println((m1 eq summon[Mirror.ProductOf[L]], m1.fromProduct(Tuple1(3))))
  case class G[T](t: T)
  println(summon[Mirror.ProductOf[G[String]]].fromProduct(Tuple1("g")))
  val f = (n: Int) =>
    case class E()
    summon[Mirror.ProductOf[E]].fromProduct(EmptyTuple).toString
  println(f(1))
  println(new K().m)
