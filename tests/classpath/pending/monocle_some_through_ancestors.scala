// jars: scala-library cats-kernel cats-core cats-free monocle-core monocle-macro
//> using dep dev.optics::monocle-core:3.3.0
// monocle's two-evidence `some`, called through an ancestor's type on a `PPrism` or a `PIso`,
// runs the instance's override, which returns a `PPrism`: `PSetter` declares it, `POptional` and
// `PTraversal` override it beside `Fold`'s one-evidence `some`, and the output names the two
// apart class by class (a task-list item). Written with the Scala.js test path, 2026-09-29;
// the expectation is scalac's.
import monocle.{Iso, Lens, Optional, PPrism, Prism, Setter, Traversal}

@main def run(): Unit =
  val p: Prism[Option[Option[Int]], Option[Int]] = Prism[Option[Option[Int]], Option[Int]](identity)(Some(_))
  val asOptional: Optional[Option[Option[Int]], Option[Int]] = p
  println(asOptional.some.isInstanceOf[PPrism[?, ?, ?, ?]])
  println(asOptional.some.getOption(Some(Some(3))))
  val asTraversal: Traversal[Option[Option[Int]], Option[Int]] = p
  println(asTraversal.some.isInstanceOf[PPrism[?, ?, ?, ?]])
  val asSetter: Setter[Option[Option[Int]], Option[Int]] = p
  println(asSetter.some.isInstanceOf[PPrism[?, ?, ?, ?]])
  println(asSetter.some.replace(5)(Some(Some(1))))
  val i: Iso[Option[Int], Option[Int]] = Iso.id[Option[Int]]
  val isoAsOptional: Optional[Option[Int], Option[Int]] = i
  println(isoAsOptional.some.isInstanceOf[PPrism[?, ?, ?, ?]])
  val o: Optional[Option[Option[Int]], Option[Int]] = Optional[Option[Option[Int]], Option[Int]](identity)(a => _ => Some(a))
  println(o.some.isInstanceOf[PPrism[?, ?, ?, ?]])
  println(o.some.getOption(Some(Some(8))))
  val l: Lens[(Option[Int], Int), Option[Int]] = Lens[(Option[Int], Int), Option[Int]](_._1)(a => s => (a, s._2))
  val lensAsSetter: Setter[(Option[Int], Int), Option[Int]] = l
  println(lensAsSetter.some.replace(3)((Some(1), 2)))
  println(l.some.getOption((Some(9), 2)))
