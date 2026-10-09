// jars: scala-library cats-kernel cats-core cats-free monocle-core monocle-macro
package lib

import monocle.{Focus, Lens, Optional}
import monocle.syntax.all.*

/** Optics over the jar's traits, whose `some` is declared with one evidence in `Fold` and with
  * two in `PSetter`: where a class inherits both, the alternatives are named apart while the
  * reach pass compiles it, after the classes above it were named and bridged.
  */
case class Address(street: String, city: Option[String])
case class Person(name: String, address: Option[Address])

object Model:
  val address: Lens[Person, Option[Address]] = Focus[Person](_.address)
  val city: Optional[Person, String] = address.some.andThen(Focus[Address](_.city)).some
  val street: Optional[Person, String] = address.some.andThen(Focus[Address](_.street))
  def rename(p: Person, n: String): Person = p.focus(_.name).replace(n)
