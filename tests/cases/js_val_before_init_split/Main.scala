// A val read before its initialiser runs across files of other packages: a parent reading an abstract val its
// subclass defines, and a value class field read early.
object Main { def main(args: Array[String]): Unit = { val a = new q.a.A; println(a.early.x); println(a.later.x) } }
