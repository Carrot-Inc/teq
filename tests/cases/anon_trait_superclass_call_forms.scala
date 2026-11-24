// An anonymous class whose first parent is a trait over a class calls the class's constructor in its own
// constructor (dotty's `ensureConstrCall`): through the class's secondary constructor, and with a default that
// reads a captured local, alone and with a second trait.
class A(val x: Int) { def this() = this(17) }
trait T extends A { val t: Int = 1 }
trait U { val u: Int = 2 }

object Main:
  def local(q: Int): String =
    class B(val y: Int = q + 1)
    trait S extends B { val s: Int = 1 }
    val v: S = new S {}
    val w: S = new S with U {}
    s"${v.y} ${v.s} ${w.y} ${w.s}"

  def main(args: Array[String]): Unit =
    val v: T = new T {}
    val w: T = new T with U {}
    println(s"${v.x} ${v.t}")
    println(s"${w.x} ${w.t} ${w.asInstanceOf[U].u}")
    println(local("abc".length + 39))
