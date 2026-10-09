// jars: scala-library tapir-core tapir-refined refined sttp-model sttp-shared-core sttp-shared-ws magnolia123 scala-java-time
//> using dep com.softwaremill.sttp.tapir::tapir-core:1.13.29
//> using dep com.softwaremill.sttp.tapir::tapir-refined:1.13.29
// tapir 1.13.29's `Schema` from its jar: `derives Schema` on case classes, enums and sealed
// traits (Magnolia's derivation with tapir's `SchemaMagnoliaDerivation` in the interpreter),
// nested types with `Option`, `List`, `Map`, `BigDecimal` and `java.time` fields, `Schema.derived`,
// `Schema.derivedEnumeration`, the `@description`, `@encodedName`, `@default` and `@validate`
// annotations, `Validator`s applied to values, `Schema.map`, `schemaForMap`, `Schema.string` and
// the schema of a refined type through `tapir-refined`.
package tapirschemas

import sttp.tapir.*
import sttp.tapir.Schema.annotations.*
import sttp.tapir.generic.auto.*
import sttp.tapir.codec.refined.*
import eu.timepit.refined.api.Refined
import eu.timepit.refined.numeric.Positive
import eu.timepit.refined.collection.NonEmpty
import eu.timepit.refined.string.MatchesRegex
import java.time.{Instant, LocalDate}

case class Address(street: String, city: String, zip: Option[String]) derives Schema
case class Pet(
    @description("the id") id: Int,
    @encodedName("pet_name") name: String,
    tags: List[String],
    @validate(Validator.min(0)) age: Int,
    @default(BigDecimal(1)) price: BigDecimal,
    born: LocalDate,
    seen: Instant,
    attrs: Map[String, Int],
    address: Address
) derives Schema

enum Color derives Schema:
  case Red, Green, Blue

sealed trait Shape derives Schema
case class Circle(r: Double) extends Shape
case class Rect(w: Double, h: Double) extends Shape
case object Dot extends Shape

case class Box[A](value: A, label: String)
case class Key(value: String)
case class Wrapper(id: Int) extends AnyVal

object Main:
  def main(args: Array[String]): Unit =
    val pet = summon[Schema[Pet]]
    println(pet.name)
    println(pet.schemaType.toString)
    println(summon[Schema[Address]].toString)
    val color = summon[Schema[Color]]
    println(color.name.toString + " " + color.schemaType.getClass.getSimpleName + " " + color.validator.show)
    val shape = summon[Schema[Shape]]
    println(shape.name.toString + " " + shape.schemaType.toString)
    val enumeration = Schema.derivedEnumeration[Color].defaultStringBased
    println(enumeration.schemaType.toString + " " + enumeration.validator.show + " " + enumeration.name)
    val box = Schema.derived[Box[Int]]
    println(box.name.toString + " " + box.schemaType.toString)
    val keyed = Schema.schemaForMap[Key, Int](_.value)
    println(keyed.schemaType.toString)
    println(summon[Schema[Option[List[Int]]]].toString)
    println(summon[Schema[Map[String, Address]]].schemaType.getClass.getSimpleName)
    println(summon[Schema[BigDecimal]].toString + " " + summon[Schema[Instant]].toString + " " + summon[Schema[LocalDate]].toString)
    println(Schema.string[Key].toString + " " + Schema.binary[Array[Byte]].schemaType + " " + Schema.anyObject[Key].schemaType)
    val mapped = summon[Schema[String]].map(s => Some(Key(s)))(_.value)
    println(mapped.toString)
    println(summon[Schema[Int]].as[Wrapper].description("wrapped").format("int32").deprecated(true).toString)
    println(summon[Schema[String]].asOption.toString + " " + summon[Schema[Int]].asIterable[List].schemaType + " " + summon[Schema[Int]].asArray.schemaType)
    val minMax = Validator.min(1).and(Validator.max(10))
    println(minMax.show.toString + " " + minMax(0).map(_.toString) + " " + minMax(5) + " " + minMax(11).length)
    println(Validator.pattern("[a-z]+")("abc").toString + " " + Validator.pattern("[a-z]+")("ABC").map(_.invalidValue))
    println(Validator.minLength(2)("a").map(_.validator.show).toString + " " + Validator.maxSize[Int, List](1)(List(1, 2)).length + " " + Validator.enumeration(List(1, 2)).show)
    println(Validator.nonEmptyString.show.toString + " " + Validator.positive[Int].show + " " + Validator.negative[Int].show + " " + Validator.inRange(1, 3).show)
    println(pet.schemaType.asInstanceOf[SchemaType.SProduct[Pet]].fields.map(f => f.name.encodedName + ":" + f.schema.description + ":" + f.schema.validator.show + ":" + f.schema.default.map(_._1)).mkString(" | "))
    println(pet.schemaType.asInstanceOf[SchemaType.SProduct[Pet]].fields.head.get(Pet(7, "rex", Nil, 3, BigDecimal(2), LocalDate.of(2020, 1, 2), Instant.EPOCH, Map(), Address("s", "c", None))))
    val positive = summon[Schema[Int Refined Positive]]
    println(positive.schemaType.toString + " " + positive.validator.show)
    val nonEmpty = summon[Schema[String Refined NonEmpty]]
    println(nonEmpty.schemaType.toString + " " + nonEmpty.validator.show)
    val regex = summon[Schema[String Refined MatchesRegex["[a-z]+"]]]
    println(regex.validator.show)
    val positiveCodec = summon[Codec[String, Int Refined Positive, CodecFormat.TextPlain]]
    println(positiveCodec.decode("5").toString + " " + positiveCodec.decode("-1").toString + " " + positiveCodec.decode("x").getClass.getSimpleName)
