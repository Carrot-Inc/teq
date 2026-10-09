// jars: scala-library zio-json magnolia zio
//> using dep dev.zio::zio-json:0.9.2
// The `Json` AST and `JsonCursor` from the jar, whose values are zio's `Chunk`s built through the
// lean std: `toJsonAST`/`fromJsonAST` round trips over derived codecs, `Json` values built by hand,
// field access, cursor navigation with a failing path's message, equality, `toString`, the
// `zio.json.ast` decoders.
import zio.json.*
import zio.json.ast.{Json, JsonCursor}

case class Address(street: String, city: String, zip: Option[String]) derives JsonCodec
case class Person(name: String, age: Int, address: Address, tags: List[String], scores: Map[String, Int]) derives JsonCodec

object Main:
  def main(args: Array[String]): Unit =
    val p = Person("Ann", 30, Address("1 Main", "Springfield", None), List("a", "b"), Map("x" -> 1))
    val ast = p.toJsonAST
    println(ast)
    println(ast.flatMap(_.as[Person]))
    val byHand = Json.Obj("name" -> Json.Str("Bob"), "n" -> Json.Num(2), "ok" -> Json.Bool(true), "none" -> Json.Null, "xs" -> Json.Arr(Json.Num(1), Json.Str("s")))
    println(byHand)
    println(byHand.toJson)
    println(byHand.toJsonPretty)
    println(byHand.get(JsonCursor.field("name")))
    println(byHand.get(JsonCursor.field("xs").isArray.element(1)))
    println(byHand.get(JsonCursor.field("missing")))
    println(byHand.get(JsonCursor.field("xs").isArray.element(5)))
    println(byHand.get(JsonCursor.field("name").isNumber))
    println(byHand.fields.map(_._1))
    println(byHand.get(JsonCursor.field("n")).map(_.asInstanceOf[Json.Num].value))
    println("""{"a":[1,2,{"b":null}],"c":"d"}""".fromJson[Json])
    println("""{"a":[1,2,{"b":null}],"c":"d"}""".fromJson[Json.Obj])
    println("""[1,2]""".fromJson[Json.Arr])
    println("""1.5""".fromJson[Json.Num])
    println("""[1,2]""".fromJson[Json.Obj])
    println(Json.Obj("a" -> Json.Num(1)) == Json.Obj("a" -> Json.Num(1)))
    println(Json.Arr(Json.Num(1)) == Json.Arr(Json.Num(2)))
    println(Json.Obj("a" -> Json.Num(1)).merge(Json.Obj("b" -> Json.Num(2))))
    println(Json.Num(1).foldUp(0)((acc, j) => acc + 1))
    println(Json.Obj("a" -> Json.Obj("b" -> Json.Num(1))).get(JsonCursor.field("a").isObject.field("b")))
    println(Json.Obj("a" -> Json.Num(1)).delete(JsonCursor.field("a")))
    println(Json.Obj().isEmpty)
    println(Json.Arr(Json.Num(1), Json.Num(2)).elements.size)
    println(Json.Str("hello").asString)
    println(Json.Num(java.math.BigDecimal.valueOf(2.5)).asNumber)
    println(Json.Bool(false).asBoolean)
    println(ast.map(_.asObject.flatMap(_.get("address")).flatMap(_.asObject).map(_.keys)))
