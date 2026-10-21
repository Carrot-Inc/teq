// A placeholder's parameter in an annotation's argument is no reference to an evidence written with its text: the
// evidence named `_$0` stays after the `Int` clause (`Desugar.referencesName` compares names by identity; the review of
// `b23cfe2a3`, `PlaceholderEvidence`).
class ann(val x: Any) extends scala.annotation.StaticAnnotation
type F = [X: Ordering as _$0] => (Int @ann(List(1).map(_ + 1))) => Int
val f: F = [X] => (x: Int) => (o: Ordering[X]) ?=> x + 1

@main def run(): Unit =
  println(f[String](41)(using Ordering.String))
