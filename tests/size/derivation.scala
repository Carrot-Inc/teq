// Derivation in the shape of Magnolia's: an inline method expanded once per field of every
// derived record, each expansion creating two single-method anonymous classes (a lazy type-class
// thunk and a default evaluator behind a serializable function trait) that the JavaScript output
// writes as closures rather than as classes with registrations.
import scala.compiletime.{constValue, erasedValue, summonInline}
import scala.deriving.Mirror

trait SerializableFunction0[+R] extends Function0[R] with java.io.Serializable:
  def apply(): R

trait Show[A]:
  def show(a: A): String

object Show:
  given Show[Int] with
    def show(a: Int): String = a.toString
  given Show[String] with
    def show(a: String): String = "\"" + a + "\""
  given Show[Boolean] with
    def show(a: Boolean): String = if a then "yes" else "no"
  given [A](using s: Show[A]): Show[Option[A]] with
    def show(o: Option[A]): String = o.fold("-")(s.show)
  given [A](using s: Show[A]): Show[List[A]] with
    def show(as: List[A]): String = as.map(s.show).mkString("[", ", ", "]")

final class Field[A](val label: String, val index: Int, tc: SerializableFunction0[Show[A]], default: SerializableFunction0[Option[A]]):
  lazy val typeclass: Show[A] = tc()
  def fallback: Option[A] = default()
  def render(a: A): String = label + "=" + typeclass.show(a)

object Fields:
  val returningNone: SerializableFunction0[Option[Nothing]] = new SerializableFunction0[Option[Nothing]]:
    def apply(): Option[Nothing] = None

  inline def field[A, L](index: Int, defaults: Map[String, () => Any]): Field[A] =
    val label = constValue[L].asInstanceOf[String]
    val tc: SerializableFunction0[Show[A]] = new SerializableFunction0[Show[A]]:
      def apply(): Show[A] = summonInline[Show[A]]
    val d: SerializableFunction0[Option[A]] = defaults.get(label) match
      case Some(evaluator) =>
        new SerializableFunction0[Option[A]]:
          def apply(): Option[A] =
            val v = evaluator()
            if v.isInstanceOf[A] then Some(v.asInstanceOf[A]) else None
      case None => returningNone.asInstanceOf[SerializableFunction0[Option[A]]]
    new Field[A](label, index, tc, d)

  inline def fields[Types <: Tuple, Labels <: Tuple](index: Int, defaults: Map[String, () => Any]): List[Field[?]] =
    inline (erasedValue[Types], erasedValue[Labels]) match
      case (_: (t *: ts), _: (l *: ls)) => field[t, l](index, defaults) :: fields[ts, ls](index + 1, defaults)
      case (_: EmptyTuple, _: EmptyTuple) => Nil

trait Record[A]:
  def fields: List[Field[?]]
  def render(a: A): String

object Record:
  inline def derived[A <: Product](using m: Mirror.ProductOf[A]): Record[A] =
    val defaults: Map[String, () => Any] = Map("note" -> (() => "n/a"), "count" -> (() => 1), "tags" -> (() => List("x")))
    val fs = Fields.fields[m.MirroredElemTypes, m.MirroredElemLabels](0, defaults)
    new Record[A]:
      val fields: List[Field[?]] = fs
      def render(a: A): String =
        val values = a.productIterator.toList
        fields.zip(values).map { case (f, v) => f.asInstanceOf[Field[Any]].render(v) }.mkString(", ")

case class Address(street: String, number: Int, note: Option[String]) derives Record
case class Person(name: String, age: Int, active: Boolean, address: Address, tags: List[String]) derives Record
case class Order(id: String, count: Int, items: List[String], gift: Boolean, note: Option[String], recipient: String) derives Record
case class Warehouse(code: String, capacity: Int, open: Boolean, manager: Option[String], zones: List[Int], note: Option[String], count: Int) derives Record
case class Shipment(order: String, warehouse: String, boxes: Int, express: Boolean, tags: List[String], note: Option[String], count: Int, weight: Int) derives Record

given Show[Address] with
  def show(a: Address): String = "Address(" + summon[Record[Address]].render(a) + ")"

object Main:
  def main(args: Array[String]): Unit =
    val address = Address("Main St", 12, None)
    println(summon[Record[Address]].render(address))
    println(summon[Record[Person]].render(Person("Ada", 36, true, address, List("a", "b"))))
    println(summon[Record[Order]].render(Order("o-1", 3, List("pen"), false, Some("fragile"), "Bob")))
    println(summon[Record[Warehouse]].render(Warehouse("W1", 500, true, Some("Cy"), List(1, 2), None, 7)))
    println(summon[Record[Shipment]].render(Shipment("o-1", "W1", 2, true, Nil, None, 0, 15)))
    for r <- List[Record[?]](summon[Record[Order]], summon[Record[Warehouse]], summon[Record[Shipment]]) do
      println(r.fields.map(f => f.label + ":" + f.fallback.getOrElse("none")).mkString(" "))
