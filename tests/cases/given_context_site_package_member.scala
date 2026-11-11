// Two searches of one package context read the package object's inherited `inst`, `S`'s from inside a class
// that inherits it too: the second may not take the first's answer from a list the context kept (scalac 3.8.4
// prints 1).
package p
class TC[A](val n: Int)
object TC { given TC[Int] = new TC(0) }
trait Inst[A] { given inst: TC[A] = new TC(1) }
package object lib extends Inst[Int]
package lib {
  object Outer {
    object S extends Inst[String] { val warm = summon[TC[Int]] }
    val result: Int = summon[TC[Int]].n
  }
  @main def main(): Unit = println(Outer.result)
}
