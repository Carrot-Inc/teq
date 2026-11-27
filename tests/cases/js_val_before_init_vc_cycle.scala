// A value class's companion object reads, while it initialises, a field of a class that holds the value class and is
// not set yet.
class V(val x: Int) extends AnyVal { def read: Int = x }
object V { println("companion"); println((new A).early.x) }
class A { def read: V = later; val early: V = read; val later: V = new V(21) }
object Main { def main(args: Array[String]): Unit = { val a = new A; println(a.early.read); println(a.later.read) } }
