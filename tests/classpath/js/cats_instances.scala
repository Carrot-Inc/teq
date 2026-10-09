// jars: scala-library cats-kernel-sjs cats-core-sjs
// cats-kernel's instances for the std's types: Eq, Order, Show, Monoid and Semigroup for Int,
// Long, Double, String, Boolean, Option, List, Vector, Map, Set, tuples, Either and BigDecimal,
// and what the syntax makes of them.
//> using dep org.typelevel::cats-core:2.13.0
import cats.*
import cats.syntax.all.*
import cats.instances.order.*

case class Slot(index: Int, label: String)
object Slot:
  given Order[Slot] = Order.by(_.index)
  given Show[Slot] = Show.show(s => s"Slot(${s.index}, ${s.label})")

object Main:
  def p(xs: Any*): Unit = println(xs.mkString(" "))
  def main(args: Array[String]): Unit =
    p(Eq[Int].eqv(1, 1), Eq[String].eqv("a", "b"), Eq[Boolean].neqv(true, false), Eq[Long].eqv(1L, 1L), Eq[Double].eqv(1.5, 1.5))
    p((List(1, 2) === List(1, 2)), (Vector(1) =!= Vector(2)), (Option(1) === Option(1)), (Map(1 -> "a") === Map(1 -> "a")), (Set(1, 2) === Set(2, 1)))
    p(((1, "a") === (1, "a")), ((1, "a", true) =!= (1, "a", false)), ((Right(1): Either[String, Int]) === Right(1)), ((Left("e"): Either[String, Int]) === Right(1)))
    p((BigDecimal("1.50") === BigDecimal("1.5")), (BigInt(3) === BigInt(3)), (Slot(1, "a") === Slot(1, "b")), (Slot(1, "a") =!= Slot(2, "a")))
    p(Order[Int].compare(1, 2), Order[String].compare("b", "a"), Order[Long].max(3L, 5L), Order[Double].min(1.5, 0.5), Order[Boolean].compare(false, true))
    p(Order[Option[Int]].compare(None, Some(1)), Order[List[Int]].compare(List(1, 2), List(1, 3)), Order[(Int, String)].compare((1, "b"), (1, "a")), Order[Either[Int, String]].compare(Left(1), Right("a")))
    p((1 < 2), ("b" > "a"), (3 max 4), Slot(1, "a").compare(Slot(2, "b")), (Slot(1, "a") < Slot(2, "b")))
    p(List(Slot(3, "c"), Slot(1, "a"), Slot(2, "b")).sorted.map(_.index), List(Slot(3, "c"), Slot(1, "a")).max.index, List(Slot(3, "c"), Slot(1, "a")).sortBy(identity).map(_.label), List(3, 1, 2).sorted(using Order[Int].toOrdering))
    p(List(Slot(2, "b"), Slot(1, "a")).minimumOption, List(Slot(2, "b"), Slot(1, "a")).maximumOption, List(2, 1).minimumByOption(-_), Order.by[Slot, String](_.label).compare(Slot(1, "b"), Slot(2, "a")))
    p(Order[Int].toOrdering.compare(1, 2), Order.fromOrdering[String].compare("a", "b"), Order.reverse(Order[Int]).compare(1, 2), Order[Int].partialCompare(1, 1).toInt, PartialOrder[Double].partialCompare(1.0, Double.NaN).isNaN)
    p(1.show, 2L.show, 1.5.show, true.show, 'c'.show, "s".show, ().show, BigDecimal("2.50").show, BigInt(7).show)
    p(Option(1).show, (None: Option[Int]).show, List(1, 2).show, Vector("a").show, Set(1).show, Map(1 -> "a").show, (1, "b").show, (Right(1): Either[String, Int]).show)
    p(List(Slot(1, "a")).show, Option(Slot(2, "b")).show, Map("k" -> Slot(3, "c")).show, (Slot(1, "a"), 2).show, Show[Slot].show(Slot(4, "d")))
    p(Show.fromToString[Slot].show(Slot(5, "e")), Show[Int].contramap[Slot](_.index).show(Slot(6, "f")), Show.show[Slot](_.label).show(Slot(7, "g")))
    p(Monoid[Int].empty, Monoid[String].empty.isEmpty, Monoid[List[Int]].empty, Monoid[Option[Int]].empty, Monoid[Map[String, Int]].empty, Monoid[Set[Int]].empty, Monoid[Vector[Int]].empty, Monoid[(Int, String)].empty)
    p((1 |+| 2), (1L |+| 2L), (1.5 |+| 2.25), ("a" |+| "b"), (List(1) |+| List(2)), (Vector(1) |+| Vector(2)), (Set(1) |+| Set(2)), (Option(1) |+| None), (Option(1) |+| Option(2)))
    p((Map("a" -> 1, "b" -> 2) |+| Map("b" -> 3, "c" -> 4)), ((1, "a") |+| (2, "b")), (BigDecimal("1.5") |+| BigDecimal("2.5")), (BigInt(1) |+| BigInt(2)), (Option(List(1)) |+| Option(List(2))))
    p(Monoid[Int].combineAll(List(1, 2, 3)), Monoid[String].combineAll(List("a", "b")), List(Option(1), None, Option(3)).combineAll, List(Map("a" -> 1), Map("a" -> 2)).combineAll, List.empty[Int].combineAll, Monoid[Map[String, List[Int]]].combineAll(List(Map("x" -> List(1)), Map("x" -> List(2)))))
    p(Semigroup[Int].combine(1, 2), Semigroup[String].combineN("ab", 3), Semigroup[List[Int]].combineAllOption(List(List(1), List(2))), Semigroup[Int].combineAllOption(Nil), Semigroup[Option[Int]].combine(Option(1), Option(2)), Semigroup.instance[Int](_ * _).combine(3, 4))
    p(Monoid[Int].isEmpty(0), Monoid[String].isEmpty("a"), Monoid.instance[Int](1, _ * _).combineAll(List(2, 3)), Monoid[Int].combineN(3, 4), Monoid[List[Int]].combineN(List(1), 2), 1.isEmpty, "".isEmpty)
    p(Hash[Int].hash(5), (Hash[String].hash("a") == "a".hashCode), (Hash[List[Int]].hash(List(1)) == Hash[List[Int]].hash(List(1))), (Hash[Option[Int]].hash(Some(1)) == Hash[Option[Int]].hash(Some(1))), (Hash[(Int, Int)].hash((1, 2)) == Hash[(Int, Int)].hash((1, 2))))
    p(Eq.fromUniversalEquals[Slot].eqv(Slot(1, "a"), Slot(1, "a")), Eq.by[Slot, Int](_.index).eqv(Slot(1, "a"), Slot(1, "b")), Eq.instance[Slot](_.label == _.label).eqv(Slot(1, "a"), Slot(2, "a")), Eq[Slot].eqv(Slot(1, "a"), Slot(1, "z")), Eq.allEqual[Slot].eqv(Slot(1, "a"), Slot(9, "z")))
    p(Order[Slot].max(Slot(1, "a"), Slot(2, "b")), Order[Slot].toOrdering.lt(Slot(1, "a"), Slot(2, "b")), Order.whenEqual(Order[Slot], Order.by[Slot, String](_.label)).compare(Slot(1, "b"), Slot(1, "a")), (Slot(1, "a") max Slot(3, "c")))
    p(Show[Option[List[Slot]]].show(Some(List(Slot(1, "a")))), Show[Map[Slot, Int]].show(Map(Slot(1, "a") -> 1)), Show[Vector[Option[Int]]].show(Vector(Some(1), None)), Show[Either[Slot, Int]].show(Left(Slot(1, "a"))))
    p(Eq[Set[Int]].eqv(Set(1, 2), Set(2, 1)), Eq[Map[String, Option[Int]]].eqv(Map("a" -> Some(1)), Map("a" -> Some(1))), Eq[Vector[List[Int]]].eqv(Vector(List(1)), Vector(List(1))), Eq[Option[Slot]].eqv(Some(Slot(1, "a")), Some(Slot(1, "b"))), Eq[Char].eqv('a', 'a'), Eq[Unit].eqv((), ()))
    p(List(1, 2, 3).foldMap(i => (i, i.toString)), List("a", "b").foldMap(s => Map(s -> 1)), List(1, 2).foldMap(Option(_)), Vector(1, 2).foldMap(List(_)), Option(3).foldMap(_ * 2), List(Set(1), Set(2)).combineAll)
    p(Monoid[Int].combine(Monoid[Int].empty, 5), Semigroup[Map[Int, Int]].combine(Map(1 -> 1), Map(1 -> 2)), Monoid[Option[String]].combineAll(List(Some("a"), Some("b"))))
    p(Invariant[Show].imap(Show[Int])(i => Slot(i, "x"))(_.index).show(Slot(3, "q")), Contravariant[Show].contramap(Show[Int])((s: Slot) => s.index).show(Slot(4, "q")), Invariant[Monoid].imap(Monoid[Int])(_.toString)(_.toInt).combine("1", "2"))
