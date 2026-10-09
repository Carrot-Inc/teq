// Derivation in the shape of tapir's schemas over Magnolia: an inline `derived` makes, at every
// derived type, an anonymous class of generic members that know nothing of the type, and hands
// it a context built by inline steps per field. The class is written once for all the types,
// and the steps once for all the fields.
import scala.compiletime.{constValue, erasedValue, summonInline}
import scala.deriving.Mirror

trait Encoder[A]:
  def encode(a: A): String

object Encoder:
  given Encoder[Int] with
    def encode(a: Int): String = a.toString
  given Encoder[String] with
    def encode(a: String): String = "\"" + a + "\""
  given Encoder[Boolean] with
    def encode(a: Boolean): String = if a then "true" else "false"
  given [A](using e: Encoder[A]): Encoder[List[A]] with
    def encode(as: List[A]): String = as.map(e.encode).mkString("[", ",", "]")

final class Param(val label: String, val index: Int, encoder: () => Encoder[Any], val optional: Boolean):
  lazy val typeclass: Encoder[Any] = encoder()

final class Context(val name: String, val params: List[Param])

trait Deriver:
  def join(ctx: Context): Encoder[Product]

object Derive:
  inline def param[A, L](index: Int): Param =
    val label = constValue[L].asInstanceOf[String]
    val enc = () => summonInline[Encoder[A]].asInstanceOf[Encoder[Any]]
    val optional = label.startsWith("maybe")
    new Param(label, index, enc, optional)

  inline def params[Ts <: Tuple, Ls <: Tuple](index: Int): List[Param] =
    inline erasedValue[(Ts, Ls)] match
      case _: (t *: ts, l *: ls) => param[t, l](index) :: params[ts, ls](index + 1)
      case _ => Nil

  inline def derived[A <: Product](using m: Mirror.ProductOf[A]): Encoder[A] =
    val deriver = new Deriver:
      def join(ctx: Context): Encoder[Product] = new Encoder[Product]:
        def encode(p: Product): String =
          val fields = ctx.params.map { param =>
            val value = p.productElement(param.index)
            val shown = if param.optional && value == null then "null" else param.typeclass.encode(value)
            quote(param.label) + ":" + shown
          }
          ctx.name + fields.mkString("{", ",", "}")
      def quote(s: String): String = "\"" + s.replace("\"", "\\\"") + "\""
    deriver.join(Context(constValue[m.MirroredLabel], params[m.MirroredElemTypes, m.MirroredElemLabels](0))).asInstanceOf[Encoder[A]]

case class Address(street: String, number: Int, city: String)
case class Person(name: String, age: Int, active: Boolean, tags: List[String])
case class Order(id: String, count: Int, items: List[String], gift: Boolean, recipient: String)
case class Warehouse(code: String, capacity: Int, open: Boolean, zones: List[Int], manager: String)
case class Shipment(order: String, warehouse: String, boxes: Int, express: Boolean, tags: List[String], weight: Int)
case class Invoice(number: String, total: Int, paid: Boolean, lines: List[String], customer: String, due: Int)

object Main:
  given Encoder[Address] = Derive.derived
  given Encoder[Person] = Derive.derived
  given Encoder[Order] = Derive.derived
  given Encoder[Warehouse] = Derive.derived
  given Encoder[Shipment] = Derive.derived
  given Encoder[Invoice] = Derive.derived

  def main(args: Array[String]): Unit =
    println(summon[Encoder[Address]].encode(Address("Main St", 12, "Riga")))
    println(summon[Encoder[Person]].encode(Person("Ada", 36, true, List("a", "b"))))
    println(summon[Encoder[Order]].encode(Order("o1", 2, List("x"), false, "Bo")))
    println(summon[Encoder[Warehouse]].encode(Warehouse("w1", 100, true, List(1, 2), "Cy")))
    println(summon[Encoder[Shipment]].encode(Shipment("o1", "w1", 3, true, Nil, 12)))
    println(summon[Encoder[Invoice]].encode(Invoice("i1", 50, false, List("l1", "l2"), "Di", 30)))
