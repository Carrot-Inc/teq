// jars: scala-library zio-json magnolia zio
//> using dep dev.zio::zio-json:0.9.2
// zio-json's annotations through Magnolia's parameter and type annotations: field names,
// exclusion, a discriminator, hints, no extra fields, member-name formats, and a codec made
// by `transform`.
import zio.json.*
case class A(@jsonField("full_name") name: String, @jsonExclude secret: String = "x", age: Int) derives JsonCodec
@jsonDiscriminator("kind") sealed trait Pet derives JsonCodec
@jsonHint("doggo") case class Dog(bark: Int) extends Pet
case class Cat(lives: Int) extends Pet
@jsonNoExtraFields case class Strict(a: Int) derives JsonCodec
@jsonMemberNames(SnakeCase) case class Snake(fooBar: Int, bazQux: String) derives JsonCodec
case class Wrap(v: Int)
object Wrap:
  given JsonCodec[Wrap] = JsonCodec.int.transform(Wrap(_), _.v)
object Main:
  def main(args: Array[String]): Unit =
    println(A("Ann", "s", 3).toJson)
    println("""{"full_name":"Bo","age":4}""".fromJson[A])
    println((Dog(1): Pet).toJson)
    println((Cat(9): Pet).toJson)
    println("""{"kind":"doggo","bark":2}""".fromJson[Pet])
    println("""{"a":1,"b":2}""".fromJson[Strict])
    println(Snake(1, "x").toJson)
    println(Wrap(5).toJson)
    println("7".fromJson[Wrap])
