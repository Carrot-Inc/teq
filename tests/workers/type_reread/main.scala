// jars: scala-library
// Interpolated strings as patterns: `case s"..."` through the std's `StringContext.s.unapplySeq`,
// and a library's own interpolator (`ci`) through the object of that name on its implicit
// class, as scalac desugars `StringContext(parts).ci.unapplySeq`.
object Lib:
  final class CIString private (val value: String):
    override def toString: String = value
    override def equals(that: Any): Boolean = that match
      case c: CIString => value.equalsIgnoreCase(c.value)
      case _ => false
  object CIString:
    def apply(s: String): CIString = new CIString(s)
  final class CIExtractor(parts: Seq[String]):
    def unapplySeq(c: CIString): Option[Seq[CIString]] =
      StringContext.glob(parts.map(_.toLowerCase), c.value.toLowerCase).map(_.map(CIString(_)))
  implicit class CISyntax(sc: StringContext):
    def ci(args: Any*): CIString = CIString(sc.s(args*))
    def ci: CIExtractor = CIExtractor(sc.parts)

import Lib.*

def describe(s: String): String = s match
  case s"$name=$value" => s"pair $name -> $value"
  case s"[$inner]" => s"bracketed $inner"
  case s"$a-$b-$c" => s"three $a $b $c"
  case s"pre$rest" => s"prefixed $rest"
  case s"${x}" if x.startsWith("z") => "z-word " + x
  case _ => "other"

def version(s: String): String = s match
  case s"$major.$minor.$patch" => s"v$major/$minor/$patch"
  case s"$major.$minor" => s"v$major/$minor"
  case other => other

def header(h: CIString): String = h match
  case ci"content-$kind" => s"content kind $kind"
  case ci"accept" => "accept"
  case _ => "?"

@main def run(): Unit =
  println(describe("a=b"))
  println(describe("[x]"))
  println(describe("1-2-3"))
  println(describe("prefix"))
  println(describe("plain"))
  println(describe("x=y=z"))
  println(version("3.8.4"))
  println(version("2.13"))
  println(version("nope"))
  println(header(CIString("Content-Type")))
  println(header(CIString("ACCEPT")))
  println(header(CIString("Host")))
  val s"$k:$v" = "key:val": @unchecked
  println(k + " " + v)
  List("a=1", "b", "c=3").collect { case s"$n=$x" => n + x }.foreach(println)
  "" match
    case s"$all" => println("empty [" + all + "]")
