// jars: scala-library zio-json magnolia zio
//> using dep dev.zio::zio-json:0.9.2
// std: lean scala-library
// zio-json's derivation from the jar: `derives JsonCodec` is `DeriveJsonCodec.gen`, a Magnolia
// derivation whose macros run in the interpreter over the jar's TASTy bodies. A case-class tree
// with options, lists and maps, a sealed trait of case classes and a case object, an enum and
// one explicit `DeriveJsonCodec.gen`; encoding, decoding, a round trip and the error messages.
import zio.json.*

case class Address(street: String, city: String, zip: Option[String])
object Address:
  given JsonCodec[Address] = DeriveJsonCodec.gen[Address]

sealed trait Shape derives JsonCodec
case class Circle(r: Int) extends Shape
case class Rect(w: Int, h: Int) extends Shape
case object Dot extends Shape

enum Color derives JsonCodec:
  case Red, Green, Blue

case class Person(
  name: String,
  age: Int,
  address: Address,
  tags: List[String],
  scores: Map[String, Int],
  nick: Option[String],
  favourite: Color,
  shapes: List[Shape],
  home: Option[Address],
) derives JsonCodec

case class Team(name: String, lead: Person, members: List[Person], byName: Map[String, Address], flags: List[Boolean], id: Long) derives JsonCodec

object Main:
  def main(args: Array[String]): Unit =
    val ann = Person("Ann", 30, Address("1 Main", "Springfield", None), List("a", "b"), Map("x" -> 1, "y" -> 2), Some("annie"), Color.Green, List(Circle(1), Rect(2, 3), Dot), None)
    val bob = Person("Bob", 41, Address("2 Side", "Shelbyville", Some("12345")), Nil, Map.empty, None, Color.Red, Nil, Some(Address("3 Back", "Ogdenville", None)))
    val team = Team("core", ann, List(bob), Map("ann" -> ann.address), List(true, false), 1234567890123L)
    val json = ann.toJson
    println(json)
    println(json.fromJson[Person])
    println(json.fromJson[Person] == Right(ann))
    val teamJson = team.toJson
    println(teamJson)
    println(teamJson.fromJson[Team] == Right(team))
    println(team.toJsonPretty)
    val shapes: List[Shape] = List(Circle(1), Rect(2, 3), Dot)
    println(shapes.toJson)
    println(shapes.toJson.fromJson[List[Shape]])
    println(Color.values.toList.toJson)
    println("[\"Blue\",\"Red\"]".fromJson[List[Color]])
    println(Address("x", "y", Some("z")).toJson)
    println("""{"street":"x","city":"y"}""".fromJson[Address])
    println("""{"name":"Ann","age":"x"}""".fromJson[Person])
    println("""{"name":"Ann"}""".fromJson[Person])
    println("""{"Circle":{"r":"big"}}""".fromJson[Shape])
    println("""{"Square":{"r":1}}""".fromJson[Shape])
    println("""{"name":"Ann","age":30,"address":{"street":"1 Main","city":"Springfield"},"tags":[],"scores":{},"favourite":"Purple","shapes":[],"nick":null}""".fromJson[Person])
    println("""{"name":"Ann","age":30,"address":{"street":"1 Main","city":"Springfield"},"tags":[],"scores":{},"favourite":"Blue","shapes":[],"nick":null}""".fromJson[Person])
