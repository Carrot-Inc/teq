// A match type's declared upper bound (`type M[X] <: B = X match ...`, the `MatchTypeTree`'s bound) is transformed
// with `super.transform(bounds)` (PostTyper.scala 691-699), its own application unchecked, so scalac 3.8.4 accepts
// `<: C[? >: String]` of `C[T <: Int]`.
// teq agrees.
class C[T <: Int]
type M[X] <: C[? >: String] = X match
  case Int => Nothing
@main def main(): Unit = println("bound unchecked")
