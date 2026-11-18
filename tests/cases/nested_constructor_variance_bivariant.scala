// teq: --wconf msg=Ignoring:s
// A nested class's constructor parameters are positions of the enclosing classes only up to the first class whose
// owner is a term or that is private to its instance (`VarianceChecker.relativeVariance`: bivariant beyond), never
// through a private constructor (`private[this]` is local), and a written type naming `@uncheckedVariance` or an
// alias is not held.
import scala.annotation.unchecked.uncheckedVariance

class Annotated[+A](a: A) {
  class Inner(x: A @uncheckedVariance) { def get: Any = x }
  class Bounded[B <: A @uncheckedVariance](b: B) { def get: Any = b }
  class Second { def this(a: A @uncheckedVariance) = this() }
  class Listed(xs: List[A @uncheckedVariance]) { def get: Any = xs }
  type U = A @uncheckedVariance
  class ThroughAlias(x: U) { def get: Any = x }
  def all: List[Any] = List(new Inner(a).get, new Bounded(a).get, { new Second(a); "second" }, new Listed(List(a)).get, new ThroughAlias(a).get)
}

class PrivateMid[+A](a: A) {
  private class Mid { class Inner(x: A) { def get: Any = x } }
  private object O { class Inner(x: A) { def get: Any = x } }
  def all: List[Any] = { val m = new Mid; List(new m.Inner(a).get, new O.Inner(a).get) }
}

class TermOwned[+A](a: A) {
  def local: Any = { class L { class Inner[+B](x: A) { def get: Any = x } }; val l = new L; new l.Inner[Int](a).get }
  def localVariant: Any = { class L[+B] { class Inner(x: A) { def get: Any = x } }; val l = new L[Int]; new l.Inner(a).get }
  val anon: Any = new AnyRef { class Inner[+B](x: A) { def get: Any = x }; override def toString = new Inner[Int](a).get.toString }
}

class PrivateConstructor[+A](a: A) {
  class Inner private[this] (x: A) { def get: Any = x }
  class Second { private[this] def this(a: A) = this() }
}

object Main {
  def main(args: Array[String]): Unit = {
    println(new Annotated("a").all.mkString(","))
    println(new PrivateMid("b").all.mkString(","))
    val t = new TermOwned("c")
    println(List(t.local, t.localVariant, t.anon).mkString(","))
    println(new PrivateConstructor("d").getClass.getSimpleName)
  }
}
