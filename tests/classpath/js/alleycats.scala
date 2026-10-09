// jars: scala-library cats-kernel-sjs cats-core-sjs alleycats-core
// alleycats-core from its jar, kittens' dependency: `Empty`, `Pure`, `Extract`, `EmptyK` and
// the instances for the std's types.
//> using dep org.typelevel::cats-core:2.13.0
//> using dep org.typelevel::alleycats-core:2.13.0
import alleycats.*
import alleycats.syntax.all.*
import alleycats.std.all.*
import cats.syntax.all.*

case class Slot(index: Int, label: String)
object Slot:
  given Empty[Slot] = Empty(Slot(0, ""))
  given cats.Eq[Slot] = cats.Eq.fromUniversalEquals

object Main:
  def p(xs: Any*): Unit = println(xs.mkString(" "))
  def main(args: Array[String]): Unit =
    p(Empty[Int].empty, Empty[String].empty.isEmpty, Empty[List[Int]].empty, Empty[Option[Int]].empty, Empty[Map[String, Int]].empty, Empty[Slot].empty, Empty[Set[Int]].empty)
    p(Empty[Int].isEmpty(0), Empty[String].nonEmpty("a"), Empty[Slot].isEmpty(Slot(0, "")), Empty.fromEmptyK[List, Int].empty)
    p(Pure[Option].pure(1), Pure[List].pure("a"), 3.pure[Option], Extract[cats.Id].extract(4), EmptyK[List].empty[Int], EmptyK[Option].empty[String])
    p(Empty[(Int, String)].empty, Empty[Vector[Int]].empty, Empty[Double].empty == 0.0, Empty[Long].empty, Empty[Unit].empty, Empty[Slot].empty.label.isEmpty)
