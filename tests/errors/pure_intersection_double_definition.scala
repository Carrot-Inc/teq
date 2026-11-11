// `(Z & Pure) & W` erases as `W` (TypeErasure.scala:801-804), so it clashes with `f(x: W)` after erasure.
// expect: Conflicting definitions:
// expect: def f(x: W): String in object A at line 8
trait Z
trait W
object A:
  def f(x: (Z & scala.caps.Pure) & W): String = "zpw"
  def f(x: W): String = "w"
@main def run(): Unit = println(A.f(null: W))
