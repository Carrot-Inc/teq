// A match type's case bodies are transformed under `Mode.Pattern` with its patterns (`transformIgnoringBoundsCheck`
// maps the whole `CaseDef`, PostTyper.scala 691-699), so their applications are not bounds-checked either (Checking.scala
// 125). scalac 3.8.4 accepts `case Int => List[C[? >: String]]` of `C[T <: Int]`; teq agrees since round
// 22 (`Worker::in_pattern_mode`).
class C[T <: Int]
type M[X] = X match
  case Int => List[C[? >: String]]
  case _ => C[? >: Boolean]
@main def main(): Unit =
  val m: M[Int] = Nil
  println(m.size)
