// A union's erasure erases each side first (`TypeErasure`'s `OrType` arm): a polymorphic function type is its
// function class there (`eraseRefinedFunctionApply`), so its union with `Null`, or with a type bounded by it, erases
// to `Function1` or `Function2` and takes the function value as it is (scalac's `id(scala.Function1)`).
object P:
  def id[N <: Null](f: ([A] => A => A) | N): [A] => A => A = f
  def pair(f: ([A] => (A, A) => A) | Null): [A] => (A, A) => A = f
  def main(args: Array[String]): Unit =
    val f = [A] => (a: A) => a
    val first = [A] => (a: A, b: A) => a
    println(id[Null](f)(1))
    println(pair(first)("a", "b"))
    println(pair(null) == null)
