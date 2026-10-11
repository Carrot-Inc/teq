// teq: --werror
// The `apply` dotty inserts on a case class's companion named where a function is expected is
// warned (Typer.adaptNoArgsOther, `caseCompanion`), named by its static path; a companion that is a
// function already, or one named where no function is expected, gets none, and neither does one
// applied to types (`F2[Int]`, the `apply` dotty's `tryInsertApplyOrImplicit` selects).
case class F(n: Int)
case class C(n: Int)
object C:
  def apply(s: String): C = C(s.length)
case class H(n: Int)
case class F2[A](a: A)
object H extends (Int => H):
  def apply(n: Int): H = new H(n)
object Outer:
  case class In(n: Int)
def run() =
  val g: Int => F = F
  val xs = List("ab").map(C)
  val o = Outer
  val i: Int => Outer.In = Outer.In
  val h: Int => H = H
  val e = F.apply(1)
  val t: Int => F2[Int] = F2[Int]
// expect: 17:21: warning: The method `apply` is inserted. The auto insertion will be deprecated, please write `F.apply` explicitly.
// expect: 18:27: warning: The method `apply` is inserted. The auto insertion will be deprecated, please write `C.apply` explicitly.
// expect: 20:34: warning: The method `apply` is inserted. The auto insertion will be deprecated, please write `Outer.In.apply` explicitly.
// absent: .scala:21:
// absent: .scala:22:
// absent: .scala:23:
