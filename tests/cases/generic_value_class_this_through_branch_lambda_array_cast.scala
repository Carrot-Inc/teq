// A generic value class's `this` in a local class, carried through a branch's join, a lambda's capture, a block, an
// array's element and two casts, instantiated at Int, at String and at another value class: each read gives the
// field it was made with (scalac: 5, hi, 7 for the first branch, 6, bye, 8 for the second).
class W[A](val k: A) extends AnyVal {
  def f(other: W[A], b: Boolean): Any = {
    class L {
      def get: Any = {
        val x: W[A] = if (b) W.this else other
        val g: () => W[A] = () => { val y = x; y }
        val arr = Array[W[A]](g())
        arr(0).asInstanceOf[Any].asInstanceOf[W[A]]
      }
    }
    new L().get
  }
}
class V(val k: Int) extends AnyVal
object Main {
  def main(args: Array[String]): Unit = {
    for (b <- List(true, false)) {
      println(new W[Int](5).f(new W[Int](6), b).asInstanceOf[W[Int]].k)
      println(new W[String]("hi").f(new W[String]("bye"), b).asInstanceOf[W[String]].k)
      println(new W[V](new V(7)).f(new W[V](new V(8)), b).asInstanceOf[W[V]].k.k)
    }
  }
}
