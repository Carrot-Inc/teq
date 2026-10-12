//> using dep com.softwaremill.macwire::macros:2.6.7
// jars: macwire-macros
// Untyped lazy vals wired by macwire, as lila's Env wires its components: dotty types each wire[T]
// call by its result T and expands it once the vals' types are known (Inlines.needsInlining after the
// typer), so the macro finds every val by its type, the val being typed among them.
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
  lazy val a = new A
  lazy val b = wire[B]
  lazy val c = wire[C]

final class Env(a: A, b: B, name: String):
  lazy val d = wire[D]
  lazy val c = wire[C]

object Main:
  def main(args: Array[String]): Unit =
    val m = new Module
    println(m.c)
    val env = new Env(new A, new B(new A), "n")
    println(env.d)
