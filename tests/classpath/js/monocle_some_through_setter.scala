// jars: scala-library cats-kernel cats-core cats-free monocle-core monocle-macro
//> using dep dev.optics::monocle-core:3.3.0
// monocle's two-evidence `some`, which `PSetter` declares, called through `Setter` on a `Prism`
// and a `Lens`, whose classes take `Fold`'s one-evidence `some` beside it: the call reaches a
// two-evidence `some` (which one: tests/classpath/pending/monocle_some_through_ancestors.scala).
import monocle.{Lens, Prism, Setter}

@main def run(): Unit =
  val p: Prism[Option[Option[Int]], Option[Int]] = Prism[Option[Option[Int]], Option[Int]](identity)(Some(_))
  val asSetter: Setter[Option[Option[Int]], Option[Int]] = p
  println(asSetter.some.replace(5)(Some(Some(1))))
  println(asSetter.some.modify(_ + 1)(Some(Some(1))))
  val l: Lens[(Option[Int], Int), Option[Int]] = Lens[(Option[Int], Int), Option[Int]](_._1)(a => s => (a, s._2))
  val lensAsSetter: Setter[(Option[Int], Int), Option[Int]] = l
  println(lensAsSetter.some.replace(3)((Some(1), 2)))
