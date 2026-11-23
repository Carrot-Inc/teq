// A value class's method that calls itself on another instance: under a `return` and in return position, the
// receiver's underlying value is what goes round the loop (the extension method's instance is unboxed), with an
// Int and a String underneath. On JavaScript the call stays a call (teq's row).
class V(val x: Int) extends AnyVal {
  final def vf(n: Int): Int = { if n > 0 then return new V(x + 1).vf(n - 1); x }
  final def tailForm(n: Int): Int = if n > 0 then new V(x + 1).tailForm(n - 1) else x
}
class W(val s: String) extends AnyVal {
  final def wf(n: Int): String = { if n > 0 then return new W(s + n).wf(n - 1); s }
}
object Main {
  def main(args: Array[String]): Unit = {
    println(new V(1).vf(3))
    println(new V(1).tailForm(3))
    println(new W("w").wf(3))
    println(new V(1).vf(100000))
    println(new V(1).tailForm(100000))
  }
}
