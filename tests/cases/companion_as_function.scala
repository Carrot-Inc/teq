// A case class's companion named where a function is expected stands for its `apply`, which
// dotty inserts with a warning (Typer.adaptNoArgsOther, `caseCompanion`): the constructor's own
// proxy, generic or of two parameters, and an explicit companion's overloaded `apply`, chosen by
// the expected function type. The warning is `tests/errors/companion_apply_inserted.scala`'s. One
// applied to types is its `apply` with them, expanded where it needs arguments, as any method is
// (dotty's `tryInsertApplyOrImplicit`), a function expected of it or not.
case class F(n: Int)
case class P(a: Int, b: String)
case class G[T](t: T)
case class C(n: Int)
object C:
  def apply(s: String): C = C(s.length)
object Outer:
  case class In(n: Int)
  object Inner:
    case class Box[A](a: A, n: Int)

@annotation.nowarn
@main def run() =
  val g: Int => F = F
  println(g(3).n)
  println(List(1, 2).map(F))
  val p: (Int, String) => P = P
  println(p(1, "x"))
  val q: Int => G[Int] = G
  println(q(2))
  println(List("ab").map(C))
  val r: Int => C = C
  val i: Int => Outer.In = Outer.In
  println(i(1))
  println(r(5))
  import Outer.Inner.Box as Make
  val m: (String, Int) => Outer.Inner.Box[String] = Make[String]
  println(m("ok", 7).n)
  val gi = G[Int]
  println(gi(9))
