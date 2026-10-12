//> using dep org.playframework::play-json:3.0.6
//> using platform jvm
// jars: play-json play-functional
// split-watch: jvm
// A library's case class has its selectors _1.._n (dotty's Desugar.productElemMeths, synthetic in the
// pickle), as the program's own: play-json's Json.writes resolves a field of scalachess's ByColor by
// `declaredMethod("_1")`, and a call reads it.
import play.api.libs.json.*

case class Pair(a: Int, b: String)

@main def run(): Unit =
  val s = JsSuccess(41, JsPath \ "x")
  println(s._1 + 1)
  println(s._2)
  // Read back by path: a JsValue's text takes Jackson, which the test's class path leaves out.
  val js = Json.toJson(Pair(1, "z"))(using Json.writes[Pair])
  println((js \ "a").as[Int] + 1)
  println((js \ "b").as[String])
