// `(l && r)(using ev)` where `&&` is a method of `l`: the further clause is the method's, as
// scalac reads `l.&&(r)(using ev)` (zio-test's `TestTrace.&&` takes `A <:< Boolean`); on a
// Boolean the operator stays the builtin.
final case class Trace[A](v: A):
  def &&(that: Trace[Boolean])(using ev: A <:< Boolean): Trace[Boolean] = Trace(ev(v) && that.v)
  def ||(that: Trace[Boolean])(using ev: A <:< Boolean): Trace[Boolean] = Trace(ev(v) || that.v)

@main def run(): Unit =
  val l = Trace(true)
  val r = Trace(false)
  println((l && r)(using <:<.refl[Boolean]))
  println((l || r)(using <:<.refl[Boolean]))
  println(l && r)
  var calls = 0
  def side(b: Boolean): Boolean = { calls += 1; b }
  println(side(false) && side(true))
  println(calls)
