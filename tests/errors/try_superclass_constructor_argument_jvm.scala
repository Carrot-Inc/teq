// expect: jvm backend: Sub: not supported on the JVM yet: try as an argument of a constructor
// expect: 1 construct not supported by the JVM backend
// teq: --target jvm
// command: build
// exit: 3
// A `try` with a handler as an argument of a superclass's constructor: what stands under it is the constructor's own
// `this` before the superclass's constructor ran (`uninitializedThis`). teq's JVM back end refuses the class;
// scalac 3.8.4 builds it.
object Out {
  def work(n: Int): Int = if n == 1 then throw new IllegalStateException("i") else n * 10
}
class Base(val v: Int)
class Sub(k: Int) extends Base(try Out.work(k) catch { case _: IllegalStateException => -1 })
object Main {
  def main(args: Array[String]): Unit = println(new Sub(1).v)
}
