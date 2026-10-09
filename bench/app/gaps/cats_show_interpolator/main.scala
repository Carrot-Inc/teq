//> using dep org.typelevel::cats-core::2.13.0
import cats.*
import cats.syntax.all.*
case class Slot(index: Int, label: String)
object Slot:
  given Order[Slot] = Order.by(_.index)
  given Show[Slot] = Show.show(s => s"Slot(${s.index}, ${s.label})")
given [A](using o: Order[A]): Ordering[A] = o.toOrdering
object Main:
  def main(args: Array[String]): Unit =
    val a = Slot(2, "b")
    println(show"slot ${a.index} named ${a.label} is ${a.show}")
