// A using or implicit parameter with a default takes the default where no given is found, as
// scalac does (http4s' `defaultCharset: Charset = DefaultCharset`).
final case class Cs(name: String)
object Cs:
  val Utf8 = Cs("UTF-8")

def decode(bytes: String)(implicit cs: Cs = Cs.Utf8): String = s"$bytes/${cs.name}"
def decode2(bytes: String)(using cs: Cs = Cs("latin1")): String = s"$bytes/${cs.name}"
def withGiven(): String =
  given Cs = Cs("given")
  decode("c") + " " + decode2("d")

@main def run(): Unit =
  println(decode("a"))
  println(decode2("b"))
  println(withGiven())
