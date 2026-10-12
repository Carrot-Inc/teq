// scala.io.Codec: a charset by its name, the implicit one of last resort, the conversions from a
// name and from a charset (the routes compiler of Play writes its files through `implicitly[Codec].name`).
import scala.io.Codec
import java.nio.charset.StandardCharsets

def nameOf(using c: Codec): String = c.name

@main def run(): Unit =
  println(Codec.UTF8.name)
  println(Codec.ISO8859)
  println(Codec("US-ASCII").charSet.name())
  println(implicitly[Codec].name)
  println(nameOf)
  println(nameOf(using "ISO-8859-1"))
  println(nameOf(using StandardCharsets.UTF_8))
  println(Codec.toUTF8("h\u00e9").length)
  println(Codec.fromUTF8("ok".getBytes(StandardCharsets.UTF_8)).mkString("[", ",", "]"))
