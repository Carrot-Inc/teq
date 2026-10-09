// jars: scala-library zio-json magnolia zio
//> using dep dev.zio::zio-json:0.9.2
// The rest of the codec surface the application's stand-in had: `JsonCodec.transformOrFail`,
// `JsonDecoder.mapOrFail`, `orElse`, `<>`, `JsonEncoder.contramap`, `JsonFieldEncoder/Decoder`
// for a key class, `SortedSet` and `SortedMap`, `Either` and `Option` corners, `UUID`.
import zio.json.*
import java.util.UUID
import scala.collection.SortedMap
import scala.collection.immutable.SortedSet

final case class Id(value: Int)
object Id:
  given JsonCodec[Id] = JsonCodec.int.transformOrFail(i => if i >= 0 then Right(Id(i)) else Left("negative id"), _.value)
  given JsonFieldEncoder[Id] = JsonFieldEncoder.int.contramap(_.value)
  given JsonFieldDecoder[Id] = JsonFieldDecoder.int.mapOrFail(i => if i >= 0 then Right(Id(i)) else Left("negative key"))

final case class Wrapped(tag: String)
object Wrapped:
  given JsonEncoder[Wrapped] = JsonEncoder.string.contramap(w => "w:" + w.tag)
  given JsonDecoder[Wrapped] = JsonDecoder.string.mapOrFail(s => if s.startsWith("w:") then Right(Wrapped(s.drop(2))) else Left("no w:"))

sealed trait Shape
final case class Circle(r: Int) extends Shape derives JsonCodec
final case class Square(side: Int) extends Shape derives JsonCodec
object Shape:
  given JsonDecoder[Shape] = JsonDecoder[Circle].widen[Shape].orElse(JsonDecoder[Square].widen[Shape])
  given JsonEncoder[Shape] = JsonEncoder[Circle].narrow[Circle].orElseEither(JsonEncoder[Square]).contramap[Shape] {
    case c: Circle => Left(c)
    case s: Square => Right(s)
  }

final case class Bag(ids: SortedSet[Int], byId: SortedMap[Int, String], names: Map[Id, Int], o: Option[Option[Int]], e: Either[String, Int], u: UUID, w: Wrapped) derives JsonCodec

object Main:
  def main(args: Array[String]): Unit =
    println(Id(3).toJson)
    println("7".fromJson[Id])
    println("-1".fromJson[Id])
    println(Map(Id(1) -> "a", Id(2) -> "b").toJson)
    println("""{"1":"a","2":"b"}""".fromJson[Map[Id, String]])
    println("""{"-3":"a"}""".fromJson[Map[Id, String]])
    println(Wrapped("x").toJson)
    println(""""w:y"""".fromJson[Wrapped])
    println(""""z"""".fromJson[Wrapped])
    println("""{"r":2}""".fromJson[Shape])
    println("""{"side":3}""".fromJson[Shape])
    println("""{"x":3}""".fromJson[Shape])
    println((JsonDecoder[Circle] <> JsonDecoder[Circle]).decodeJson("""{"r":9}"""))
    val bag = Bag(SortedSet(3, 1, 2), SortedMap(2 -> "two", 1 -> "one"), Map(Id(5) -> 50), Some(None), Right(4), UUID.fromString("123e4567-e89b-12d3-a456-426614174000"), Wrapped("q"))
    val text = bag.toJson
    println(text)
    println(text.fromJson[Bag])
    println(text.fromJson[Bag] == Right(bag))
    println("""{"ids":[2,2,1],"byId":{"b":"x","a":"y"},"names":{},"o":null,"e":{"Left":"no"},"u":"123e4567-e89b-12d3-a456-426614174000","w":"w:"}""".fromJson[Bag])
    println("""{"ids":[1],"byId":{},"names":{},"e":{"Right":1},"u":"not-a-uuid","w":"w:"}""".fromJson[Bag])
    println("""{"ids":[1],"byId":{},"names":{},"e":{"Both":1},"u":"123e4567-e89b-12d3-a456-426614174000","w":"w:"}""".fromJson[Bag])
    println((Some(Some(1)): Option[Option[Int]]).toJson + " " + (None: Option[Int]).toJson + " " + (Some(None): Option[Option[Int]]).toJson)
    println("null".fromJson[Option[Int]].toString + " " + "[null]".fromJson[List[Option[Int]]] + " " + "1".fromJson[Option[Int]])
    println((Left("l"): Either[String, Int]).toJson + " " + """{"Right":2}""".fromJson[Either[String, Int]])
    println(bag.toJsonPretty)
