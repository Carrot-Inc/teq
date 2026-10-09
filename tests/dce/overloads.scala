// Alternatives of an overloaded name are reached one by one: what nothing calls is dropped,
// also below a trait through which another alternative is called.
trait Channel:
  def send(text: String): String
  def send(code: Int): String
  def send(text: String, times: Int): String = send(text) * times

class Pipe extends Channel:
  def send(text: String): String = "text " + text
  def send(code: Int): String = "code " + code
  def send(flag: Boolean): String = "flag " + flag

object Codec:
  def encode(n: Int): String = "n" + n
  def encode(s: String): String = "s" + s
  def encode(n: Int, s: String): String = encode(n) + encode(s)

def shorten(s: String): String = s.take(3)
def shorten(n: Int): String = shorten(n.toString)
def shorten(xs: List[String]): String = xs.map(shorten).mkString

@main def run(): Unit =
  val c: Channel = Pipe()
  println(c.send("a"))
  println(Codec.encode(1))
  println(shorten("abcdef"))
