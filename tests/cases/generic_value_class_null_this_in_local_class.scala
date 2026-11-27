// A generic value class instantiated at String and made with `null`, its `this` returned whole from a local class:
// the value is the box (`true`), as scalac makes it where the local class's result needs it.
class W[A](val k: A) extends AnyVal {
  def f: Any = {
    class L {
      def get: Any = W.this
    }
    new L().get
  }
}
object Main {
  def main(args: Array[String]): Unit = {
    val x = new W[String](null).f
    println(x.isInstanceOf[W[?]])
  }
}
