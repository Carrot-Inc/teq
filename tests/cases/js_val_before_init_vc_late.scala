// A value class field read before its initialiser runs, the value class defined after the class that reads it.
class A { def get: V = later; val early: V = get; val later: V = new V(21) }
class V(val x: Int) extends AnyVal
object Main { def main(args: Array[String]): Unit = { val a = new A; println(a.early.x); println(a.later.x) } }
