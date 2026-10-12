//> using dep com.softwaremill.macwire::macros:2.6.7
// jars: macwire-macros
// macwire's wire[T], whose implementation (pickled by Scala 3.3) orders the scopes it searches with
// `Ordering.ordered[Scope]($conforms)`, scala-library's evidence-taking `ordered`. The vals are typed
// here; macwire_wire_untyped has them untyped.
import com.softwaremill.macwire.*

class A:
  override def toString = "A"
class B(a: A):
  override def toString = s"B($a)"
class C(a: A, b: B):
  override def toString = s"C($a, $b)"
class D(c: C, name: String):
  override def toString = s"D($c, $name)"

class Module:
  lazy val a: A = new A
  lazy val b: B = wire[B]
  lazy val c: C = wire[C]

final class Env(a: A, b: B, name: String):
  lazy val c: C = wire[C]
  lazy val d: D = wire[D]

object Main:
  def main(args: Array[String]): Unit =
    val m = new Module
    println(m.c)
    val env = new Env(new A, new B(new A), "n")
    println(env.c)
    println(env.d)
