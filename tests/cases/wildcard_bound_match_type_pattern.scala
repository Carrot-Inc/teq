// A match type's case patterns are not bounds-checked: `PostTyper` transforms each case of a `MatchTypeTree` under
// `Mode.Pattern` (`transformIgnoringBoundsCheck`, PostTyper.scala 691-699; Checking.scala 125). scalac 3.8.4
// accepts `case C[? >: String]` of `C[T <: Int]` and reduces `M[Int]` to `String` (`Int` is disjoint from `C`).
// teq agrees since round 22 (`Worker::in_pattern_mode`).
class C[T <: Int]
type M[X] = X match
  case C[? >: String] => Int
  case _ => String
@main def main(): Unit =
  val s: M[Int] = "reduced"
  println(s)
