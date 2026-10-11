// A using parameter no given is found for takes its default as dotty's `adaptNoArgsImplicitMethod`
// does: the clause applied again with the givens found as named arguments and the others left out,
// so the function part is lifted first (`needLiftFun`) and an earlier clause's default is evaluated
// once; a given found beside a hole is bound where it is not pure (`TypedApply.result`).
object U:
  def mark(s: String): Int = { print(s); 1 }
  def entry(msg: String, level: String = { print("L"); "info" })(using ctx: String = { print("C"); "main" }): String =
    s"$ctx/$level: $msg"
  def two(a: Int = mark("A"))(using x: Int = mark("X"), y: String = "y"): String = s"$a$x$y"
  class Box(v: Int = mark("V"))(using n: Int = mark("N")):
    override def toString = s"Box($v, $n)"
  def recv(): U.type = { print("R"); U }

@main def main(): Unit =
  println(U.entry("x"))
  println(U.entry("x", "lvl"))
  println(U.recv().entry("y"))
  locally {
    given String = { print("G"); "g" }
    println(U.two())
    println(U.two(5))
  }
  println(U.Box())
