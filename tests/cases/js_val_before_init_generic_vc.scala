// A generic value class field read before its initialiser runs: the erased field is a reference (null).
class W[T](val x: T) extends AnyVal
class A { def read: W[Int] = later; val early: W[Int] = read; val later: W[Int] = new W(21) }
object Main { def main(args: Array[String]): Unit = { val a = new A; println(a.early.x); println(a.later.x) } }
