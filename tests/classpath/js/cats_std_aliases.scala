// jars: cats-kernel-sjs cats-core-sjs
// The aliases of scala-library's `scala` package object that cats' instances are declared over
// (`scala.package.BigDecimal`, `BigInt`, `List`, `Vector`, `Seq`, `Map`, `Set`, `IndexedSeq`,
// `Iterable`, `StringBuilder`, `Range`, `::`, `Nil`) bind to the std's definitions without
// scala-library on the class path.
//> using dep org.typelevel::cats-core:2.13.0
import cats.*
import cats.syntax.all.*

object Money:
  extension (amount: BigDecimal)
    def pretty: String = amount.show
    def same(that: BigDecimal): Boolean = amount === that
  def total(xs: List[BigDecimal]): BigDecimal = xs.combineAll
  def sorted(xs: List[BigDecimal]): List[BigDecimal] = xs.sorted(using Order[BigDecimal].toOrdering)

object Main:
  def main(args: Array[String]): Unit =
    println(Money.pretty(BigDecimal("2.50")) + " " + Money.same(BigDecimal("1.0"))(BigDecimal("1.00")) + " " + Money.total(List(BigDecimal(1), BigDecimal("2.5"))))
    println(Money.sorted(List(BigDecimal(3), BigDecimal(1), BigDecimal(2))))
    println(show"${BigDecimal(5)} ${BigInt(7)} ${List(BigInt(1), BigInt(2))} ${Vector(1, 2)} ${Set(1)} ${Map(1 -> "a")}")
    println((BigInt(3) === BigInt(3)).toString + " " + (BigInt(1) |+| BigInt(2)) + " " + (BigDecimal(1) |+| BigDecimal(2)) + " " + Monoid[BigInt].empty + " " + Monoid[BigDecimal].empty)
    println(Order[BigInt].compare(BigInt(1), BigInt(2)).toString + " " + Order[BigDecimal].max(BigDecimal(1), BigDecimal(2)) + " " + Hash[BigInt].hash(BigInt(12)) + " " + (BigInt(5) =!= BigInt(6)))
    println((List(1, 2) |+| List(3)).toString + " " + (Vector(1) |+| Vector(2)) + " " + (1 :: Nil).show + " " + (Seq(1, 2): Seq[Int]).show + " " + Eq[List[Int]].eqv(List(1), List(1)))
    println((Map("a" -> 1) |+| Map("a" -> 2)).toString + " " + Set(1, 2).show + " " + (1 to 3).toList.show + " " + (Iterable(1, 2): Iterable[Int]).toList.combineAll)
    val sb = new StringBuilder("ab")
    println(List(BigDecimal("1.5"), BigDecimal("2.5")).sorted(using Order[BigDecimal].toOrdering).toString + " " + sb.append("c").toString + " " + List(BigDecimal(1)).show + " " + Option(BigDecimal(2)).show)
