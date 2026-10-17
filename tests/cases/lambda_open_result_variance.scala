// A lambda's result type variables by their variance in the result (`Inferencing.interpolateTypeVars`):
// a covariant one instantiated from below, a contravariant one from above, an invariant one kept open
// and, with no expected type to fix it, instantiated from below when the enclosing application ends; one
// bounded above alone is its upper bound (`upper_only`: `Inv[AnyVal]`).
package lambdaopenresultvariance

final case class Item[T](title: String, value: T, disabled: Boolean = false)
final case class CItem[+T](title: String, value: T, disabled: Boolean = false)
final case class Category(code: String, label: String)
object Category:
  val values: List[Category] = List(Category("flower", "Flower"), Category("edible", "Edible"))

class Sink[-T]:
  def take(t: T): String = "ok"
def sink[T](t: T): Sink[T] = new Sink[T]
class Inv[T]
def inv[T <: AnyVal](): Inv[T] = new Inv[T]

@main def main(): Unit =
  val covariant: List[CItem[Option[String]]] =
    CItem[Option[String]]("none", None) :: Category.values.map(c => CItem(c.label, Some(c.code)))
  println(covariant.map(_.value))
  val lists: List[List[Option[String]]] = List[Option[String]](None) :: List("x").map(s => List(Some(s)))
  println(lists)
  val sinks = List("x").map(s => sink(Some(s)))
  val wide: List[Sink[Any]] = sinks
  println(wide.head.take(1))
  val inferred = Category.values.map(c => Item(c.label, Some(c.code)))
  val same: List[Item[Some[String]]] = inferred
  println(same.map(_.value))
  val invs = List(1).map(_ => inv())
  val upper: List[Inv[AnyVal]] = invs
  println(upper.size)
