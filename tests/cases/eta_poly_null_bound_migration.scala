// A union with a type bounded by `Null` or `Nothing` (`isLowerBottomType` through `hasClassSymbol`, which follows the
// bound) derives from `PolyFunction` by its other side: a function to `Migrations.typedAsFunction`, so under
// 3.0-migration `f _` is `f`, no `() => f` wrapper, and `test[Null](f)(1)` applies the value, which conforms to the
// result type written, `Null` being below a polymorphic function type (`TypeComparer.isNullable`).
// teq: --source 3.0-migration
object P {
  def test[N <: Null](f: ([A] => A => A) | N): [A] => A => A = f _
  def bottom[N <: Nothing](f: ([A] => A => A) | N): [A] => A => A = f _
  def main(args: Array[String]): Unit = {
    val f = [A] => (a: A) => a
    println(test[Null](f)(1))
    println(bottom[Nothing](f)("b"))
  }
}
