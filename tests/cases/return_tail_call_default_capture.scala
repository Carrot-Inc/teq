// A default argument that captures an earlier parameter, left out by a call of the def by itself under a `return`:
// each round's closure sees that round's parameter (dotty's TailRec gives each iteration its own bindings and calls the
// default getter with values). On the JVM and in the interpreter the call goes round the loop; on JavaScript it is a
// call.
object Main {
  final def f(n: Int, xs: List[() => Int])(k: () => Int = () => n): String = {
    if n > 0 then return f(n - 1, k :: xs)()
    (k :: xs).map(_()).mkString(",")
  }
  def main(args: Array[String]): Unit = println(f(3, Nil)())
}
