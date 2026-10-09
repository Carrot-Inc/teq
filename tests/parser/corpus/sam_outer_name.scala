package samoutername

trait Enc[A]:
  def enc(a: A): String

trait Handler:
  def handle(msg: String): String

case class Codec[A](enc: Enc[A], name: String):
  def transform[B](g: B => A): Codec[B] = Codec(b => enc.enc(g(b)), name + "'")
  def twice: Codec[A] = Codec(a => enc.enc(a) + enc.enc(a), name)

class Server(val handle: String => String):
  def handler: Handler = msg => handle(msg) + "!"
  def nested: Handler = msg =>
    val inner: Handler = m => handle(m) + "?"
    inner.handle(msg)

@main def main(): Unit =
  val c = Codec[Int](a => a.toString, "int")
  println(c.transform[String](_.length).enc.enc("abc"))
  println(c.twice.enc.enc(4))
  val s = Server(_.toUpperCase)
  println(s.handler.handle("hi"))
  println(s.nested.handle("yo"))
