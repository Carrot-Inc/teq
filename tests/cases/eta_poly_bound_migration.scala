// A value whose type derives from `PolyFunction` (`Types.derivesFrom` of a type parameter's or an abstract member's
// bound, a union with `Null`) is a function to `Migrations.typedAsFunction`: under 3.0-migration `f _` is `f`, no
// `() => f` wrapper, so `test(f)(1)` applies the value.
// teq: --source 3.0-migration
object P {
  def test[F <: [A] => A => A](f: F) = f _
  trait C { type F <: [A] => A => A; val f: F; def g = f _ }
  def nullable(h: ([A] => A => A) | Null) = h _
  def main(args: Array[String]): Unit = {
    val f = [A] => (a: A) => a
    println(test(f)(1))
    val c = new C { type F = [A] => A => A; val f = [A] => (a: A) => a }
    println(c.g("x"))
    println(nullable(f) != null)
  }
}
