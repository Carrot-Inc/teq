// A method's or a class's type parameter's annotation is typed outside the definition's own contexts (`annotContext`'s
// `isInner`: the method's or the class's), so the clause's binders are not in scope there, and scalac 3.8.4 refuses each
// (E006); a type definition's own lambda clause sees its binders (`completerTypeParams`). teq leaves a method's and a
// class's type-parameter annotations untyped (master's: `nonexistent` there is not reported either), a part of its own
// (found by the fold of the review of `6c316c160`). scalac's lines, at their trees' points.
// expect: 11:26: error: Not found: type T
// expect: 12:26: error: Not found: type U
// expect: 14:28: error: Not found: type X
class ann(x: Any) extends scala.annotation.StaticAnnotation
object BinderAnnotScope {
  def f[@ann(Option.empty[T]) T](x: T): T = x
  def g[@ann(Option.empty[U]) T, U](x: T): T = x
  type L = [@ann(Option.empty[A]) A] =>> A
  class C[@ann(Option.empty[X]) X]
}
