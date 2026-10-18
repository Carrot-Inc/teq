// A `given` generator binds a variable (`Desugar.makeFor`'s `isVarBinding`, the `Bind` marked `Given` unwrapped to
// its `_: Int`), so before 3.4 it is no pattern to filter: `map` alone, no `withFilter`.
// teq: --source 3.3
class Box[A](val a: A) {
  def map[B](f: A => B): Box[B] = { println("map"); new Box(f(a)) }
  def withFilter(f: A => Boolean): Box[A] = { println("filter"); this }
}
object P {
  def main(args: Array[String]): Unit = {
    val r = for given Int <- new Box(1) yield summon[Int]
    println(r.a)
  }
}
