//> using platform js
// java.net.URI of the std's platform layer, as Scala.js's javalib parses, resolves and compares.
import java.net.{URI, URISyntaxException}

@main def Main(): Unit =
  val u = new URI("https://user:pw@Example.com:8080/a/b/../c/./d?q=1%20x#frag")
  println(List(u.getScheme, u.getUserInfo, u.getHost, u.getPort, u.getPath, u.getRawQuery, u.getQuery, u.getFragment).mkString("|"))
  println(u.normalize())
  println(u.resolve("../e?z#y"))
  println(u.resolve("#only"))
  println(u.resolve("//other.org/p"))
  println(new URI("mailto:a@b.c").isOpaque)
  println(new URI("rel/path").isAbsolute)
  println(new URI("http://a.com/x").relativize(new URI("http://a.com/x/y/z")))
  println(new URI("http", "h.com", "/a b", "f g"))
  println(new URI("HTTP://A.com/%aa") == new URI("http://a.com/%AA"))
  println(new URI("http://a.com/p").compareTo(new URI("http://a.com/q")) < 0)
  println(new URI("http://a.com/%aa").hashCode == new URI("HTTP://a.com/%AA").hashCode)
  println(new URI("http://a.com/é").toASCIIString)
  try new URI("http://a b") catch case e: URISyntaxException => println("syntax " + (e.getInput == "http://a b"))
  try URI.create("::") catch case e: IllegalArgumentException => println("illegal")
