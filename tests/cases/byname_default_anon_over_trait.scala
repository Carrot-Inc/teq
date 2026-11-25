// An anonymous class over a trait whose superclass's constructor takes a by-name parameter with a default, the call
// left out: the anonymous class's own constructor makes the superclass's call (the trait's superclass put first,
// dotty's `Namer.ensureFirstIsClass`), passing the default as a thunk that `HoistSuperArgs` hoists beside the class: in
// a method, in a block whose default reads the block's value, and in a class's member.
class A(x0: => Int = 5) { val x: Int = x0 }
trait T extends A { val t: Int = 1 }

class Outer:
  class B(x0: => Int = 7) { val x: Int = x0 }
  trait U extends B
  def run(): String =
    val v: B = new U { println("member body") }
    s"member ${v.x}"

object Main:
  def block(q: Int): String =
    class C(x0: => Int = q + 1) { val x: Int = x0 }
    trait V extends C
    val v: C = new V { println("block body") }
    s"block ${v.x}"

  def main(args: Array[String]): Unit =
    val a: A = new T { val k: String = if (t > 0) "p" else "n"; println(k) }
    println(s"method ${a.x}")
    println(block(8))
    println(new Outer().run())
