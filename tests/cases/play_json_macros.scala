//> using dep org.playframework::play-json:3.0.6
//> using platform jvm
// jars: play-json play-functional
// split-watch: jvm
// play-json's Json.format, reads and writes, whose implementation (pickled by Scala 3.3) summons a
// `JsonConfiguration.Aux[? <: MacroOptions]` (the alias's refinement bounded by the wildcard), names its
// fields through `f"..."`, lets a quote pattern's `t` escape its case in an inferred type, and makes an
// anonymous `OWrites` in an inline call that reads the expansion's `lazy val cfg`. A JVM library whose
// own collections the interpreter's std does not run; the values are printed without Jackson.
import play.api.libs.json.*

case class Person(name: String, age: Int, nick: Option[String])
case class Team(title: String, members: List[Person])

object Main:
  given Format[Person] = Json.format[Person]
  given Reads[Team] = Json.reads[Team]
  given OWrites[Team] = Json.writes[Team]

  def show(js: JsValue): String = js match
    case JsObject(fields) => fields.map((k, v) => s"$k=${show(v)}").mkString("{", ", ", "}")
    case JsArray(items) => items.map(show).mkString("[", ", ", "]")
    case JsString(s) => "\"" + s + "\""
    case JsNumber(n) => n.toString
    case other => other.getClass.getSimpleName

  def main(args: Array[String]): Unit =
    val p = Person("a", 3, None)
    println(show(Json.toJson(p)))
    println(Json.toJson(p).validate[Person])
    val q = Person("b", 4, Some("bee"))
    println(show(Json.toJson(q)))
    println(Json.toJson(q).validate[Person])
    val t = Team("t", List(p, q))
    println(show(Json.toJson(t)))
    println(Json.toJson(t).validate[Team])
    println(JsObject(Seq("name" -> JsString("c"))).validate[Person].isError)
