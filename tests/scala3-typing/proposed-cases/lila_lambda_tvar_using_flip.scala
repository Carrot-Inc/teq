// lila: SecureOrScopedBody: `action(parse.anyContent): req ?=> handleScopedBody(Seq.empty) { _ ?=> _ ?=> ... }`
// where A of handleScopedBody[A] is fixed only by `(using Request[A])`, and play's Request is covariant.
import scala.annotation.implicitNotFound

@implicitNotFound("Cannot find any HTTP Request here")
class Request[+A](val body: A)
class Context(val req: Request[?])
final class BodyContext[A](val body: Request[A]) extends Context(body)
class Me(val name: String)

def ifGranted(f: => String)(using ctx: Context): String = "granted " + f
def handleScopedBody[A](sel: Seq[String])(f: BodyContext[A] ?=> Me ?=> String)(using req: Request[A]): String =
  f(using BodyContext(req))(using Me("bob"))
def action[A](parser: A)(handler: Request[A] ?=> String): String = handler(using Request(parser))

def secureBody(f: BodyContext[?] ?=> Me ?=> String): String =
  action("content"): req ?=>
    handleScopedBody(Seq.empty) { _ ?=> _ ?=>
      ifGranted(f)
    }

@main def run() =
  println(secureBody(summon[BodyContext[?]].body.body.toString + " by " + summon[Me].name))
