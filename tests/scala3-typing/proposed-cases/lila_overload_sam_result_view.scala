// lila: Found(env.fide.playerApi.withFollow(id))(JsonOk) with three JsonOk overloads and
// Found[A](a)(f: A => Fu[Result]), given Conversion[Result, Fu[Result]].
// No alternative's function type is compatible with A => Fu[Result]; scalac then treats the
// reference as eta-expanded (Applications.resolveOverloaded: SAMType -> narrowByTypes), where
// the result Result may reach Fu[Result] through the conversion, and picks [A](body: A).
import scala.language.implicitConversions
case class Fu[A](a: A)
case class JsValue(s: String)
case class Result(body: String)
trait Writes[A] { def writes(a: A): JsValue }
given Conversion[Result, Fu[Result]] = Fu(_)

def JsonOk(body: JsValue): Result = Result(body.s)
def JsonOk[A: Writes](body: A): Result = JsonOk(summon[Writes[A]].writes(body))
def JsonOk[A: Writes](fua: Fu[A]): Fu[Result] = Fu(JsonOk(fua.a))

def Found[A](a: Option[A])(f: A => Fu[Result]): Fu[Result] = a.fold(Fu(Result("404")))(f)

case class Player(name: String)
given Writes[Player] = p => JsValue(p.name)

@main def run() =
  println(Found(Some(Player("magnus")))(JsonOk))
  println(Found(Option.empty[Player])(JsonOk))
