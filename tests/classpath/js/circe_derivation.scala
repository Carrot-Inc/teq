// targets: jvm
// std: scala-library
// jars: scala-library cats-kernel cats-core circe-numbers circe-core circe-generic
//> using dep io.circe::circe-generic:0.14.16
// circe 0.14.16's derivation, run on the JVM beside its jars (JavaScript and the interpreter
// lack java.lang.Character.isISOControl and java.util.LinkedHashMap, which its printer and
// objects use): `loopUnrolled` calls the abstract inline `Inliner.apply` on a parameter bound
// to the object `constString`, to `new EncoderDeriveSum` and `new DecoderDeriveSum` (classes
// with a `using` clause) chosen by an `inline if`, and to `new SummonSingleton[A]`; each call
// reaches the argument's own inline implementation.
import io.circe.*
import io.circe.syntax.*
import io.circe.generic.semiauto.*

case class Point(x: Int, y: Int, label: String)
sealed trait Shape
case class Circle(center: Point, radius: Double) extends Shape
case class Rect(a: Point, b: Point) extends Shape
case object Empty extends Shape

given Encoder[Point] = deriveEncoder[Point]
given Decoder[Point] = deriveDecoder[Point]
given Encoder[Shape] = deriveEncoder[Shape]
given Decoder[Shape] = deriveDecoder[Shape]

object Auto:
  import io.circe.generic.auto.*
  case class Tag(name: String, weight: Int)
  sealed trait Mode
  case class Fast(level: Int) extends Mode
  case object Idle extends Mode
  def run(): Unit =
    val json = List[Mode](Fast(3), Idle).asJson
    println(json.noSpaces)
    println(json.as[List[Mode]])
    println(Tag("t", 2).asJson.noSpaces)
    println(Json.obj("name" -> "u".asJson, "weight" -> 5.asJson).as[Tag])

@main def run(): Unit =
  val shapes: List[Shape] = List(Circle(Point(1, 2, "c"), 3.5), Rect(Point(0, 0, "a"), Point(4, 5, "b")), Empty)
  val json = shapes.asJson
  println(json.noSpaces)
  println(json.as[List[Shape]])
  println(Point(7, 8, "p").asJson.noSpaces)
  println(Json.obj("x" -> 1.asJson, "y" -> 2.asJson, "label" -> "q".asJson).as[Point])
  println(Json.obj("x" -> 1.asJson).as[Point])
  Auto.run()
