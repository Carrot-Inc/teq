// jars: scala-library chimney chimney-macro-commons scala-collection-compat scala-java-time
//> using dep io.scalaland::chimney:1.11.0
// chimney 1.11.0's `into[T]` builder from its jar: `withFieldConst`, `withFieldComputed` with a
// lambda over the source and over a field, `withFieldRenamed`, then `transform`, in the shapes an
// application writes (an optional field computed from a string, a list cut to its head, a value
// looked up through a curried helper, a field taken from a parameter).
// scala-java-time is on the class path, as the application's is: chimney's macros time their
// derivation with `java.time.Instant.now()`, which the interpreter runs from that jar.
package chimneydsl

import io.scalaland.chimney.dsl.*

final case class Draft(street: String, extra: String, notes: String, point: Option[(Double, Double)])
final case class Address(street: String, extra: Option[String], notes: Option[String], lat: Double, lng: Double)

final case class Price(label: String, amount: BigDecimal)
final case class Item(id: Long, title: String, prices: List[Price], group: String, image: Map[String, String])
final case class Preview(id: Long, title: String, prices: List[Price], section: String,
                         firstLabel: Option[String], firstAmount: Option[BigDecimal], webImage: String,
                         cursor: Option[String])

def imageOf(i: Item)(pick: Map[String, String] => Option[String]): String = pick(i.image).getOrElse("none")

def address(d: Draft): Either[String, Address] = d.point match
  case None => Left("no point")
  case Some((lat, lng)) =>
    Right(
      d.into[Address]
        .withFieldComputed(_.extra, _.extra.some.filter(_.nonEmpty))
        .withFieldComputed(_.notes, _.notes.some.filter(_.nonEmpty))
        .withFieldConst(_.lat, lat)
        .withFieldConst(_.lng, lng)
        .transform)

def preview(i: Item, cursor: Option[String]): Preview =
  i.into[Preview]
    .withFieldComputed(_.prices, _.prices.take(1))
    .withFieldRenamed(_.group, _.section)
    .withFieldComputed(_.firstLabel, _.prices.headOption.map(_.label))
    .withFieldComputed(_.firstAmount, _.prices.headOption.map(_.amount))
    .withFieldComputed(_.webImage, p => imageOf(p)(_.get("web")))
    .withFieldConst(_.cursor, cursor)
    .transform

extension [A](a: A) def some: Option[A] = Some(a)

@main def main(): Unit =
  println(address(Draft("Main St 1", "", "ring twice", Some((1.5, -2.25)))))
  println(address(Draft("Main St 1", "Apt 3", "", None)))
  val item = Item(42L, "Tea", List(Price("small", BigDecimal("2.50")), Price("large", BigDecimal("4.00"))),
    "drinks", Map("web" -> "abc"))
  println(preview(item, Some("c1")))
  println(preview(item.copy(prices = Nil, image = Map.empty), None))
