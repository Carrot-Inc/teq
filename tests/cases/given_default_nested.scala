// A given whose own using parameter has a default, found inside another search: without a
// given for the parameter the candidate takes the default, as at a direct call (http4s'
// `UrlForm.entityEncoder(implicit charset: Charset = UTF-8)` behind an `EntityEncoder`
// search), and a given in scope still wins over the default.
class Charset(val name: String)
object Charset:
  val UTF8: Charset = Charset("UTF-8")

trait Enc[F[_], A]:
  def cs: Charset

class Form
object Form:
  implicit def enc[F[_]](implicit cs0: Charset = Charset.UTF8): Enc[F, Form] = new Enc[F, Form] { def cs = cs0 }

class Page
object Page:
  given enc[F[_]](using cs0: Charset = Charset.UTF8): Enc[F, Page] with
    def cs = cs0

def encode[A](a: A)(using e: Enc[Option, A]): String = e.cs.name

@main def main(): Unit =
  println(summon[Enc[Option, Form]].cs.name)
  println(encode(new Page))
  locally {
    given Charset = Charset("latin1")
    println(encode(new Form))
  }
