// jars: scala-library cats-kernel cats-core case-insensitive
//> using dep org.typelevel::case-insensitive:1.5.0
// case-insensitive 1.5.0 from its jar: `CIString` compiled from its TASTy body (equality and
// hashing that ignore case, comparison, `contains`, `transform`), the `ci"..."` interpolator
// through the `CIStringSyntax` implicit class, and the cats instances the companion carries.
// `case ci"..."` patterns go through the `ci` object of that implicit class. Left out: `sorted`
// over `CIString` (no `Ordering` through `Ordered` in the lean std's search).
import org.typelevel.ci.*
import cats.syntax.all.*

case class Header(name: CIString, value: String)

object Main:
  def main(args: Array[String]): Unit =
    println("-- equality")
    val a = CIString("Content-Type")
    val b = CIString("content-type")
    println(a == b)
    println(a.equals(b))
    println(a == CIString("Content-Length"))
    println(a.hashCode == b.hashCode)
    println(a.hashCode == CIString("CONTENT-TYPE").hashCode)
    println(a.hashCode == CIString("Content-Length").hashCode)
    println(CIString("") == CIString.empty)
    println(CIString("ß") == CIString("SS"))
    println(CIString("İ") == CIString("i"))

    println("-- interpolator")
    val ct = ci"Content-Type"
    println(ct)
    println(ct == a)
    val name = "Accept"
    println(ci"$name-Encoding" == CIString("accept-encoding"))
    println(ci"x$name${1 + 1}y")

    println("-- members")
    println(a.toString)
    println(a.length)
    println("" + a.isEmpty + " " + CIString("").isEmpty + " " + a.nonEmpty)
    println(CIString("  padded  ").trim)
    println(a.transform(_.toUpperCase))
    println(a.contains(CIString("TYPE")))
    println(a.contains(CIString("type")))
    println(a.contains(CIString("typo")))
    println(a.contains(CIString("")))
    println(CIString("abc").contains(CIString("abcd")))
    println(a.compare(b))
    println(CIString("abc").compare(CIString("ABD")) < 0)
    println(CIString("b") > CIString("A"))
    println(CIString("a") < CIString("B"))
    println(CIString("Zed") >= CIString("zed"))

    println("-- collections")
    val headers = List(Header(CIString("Host"), "x"), Header(CIString("Accept"), "*/*"), Header(CIString("host"), "y"))
    println(headers.map(_.name).distinct)
    println(headers.filter(_.name == ci"HOST").map(_.value))
    println(Set(CIString("A"), CIString("a"), CIString("b")).size)
    println(Map(CIString("Key") -> 1).get(CIString("KEY")))

    println("-- patterns")
    def kind(h: CIString): String = h match
      case ci"content-$rest" => s"content ($rest)"
      case ci"x-$vendor-$name" => s"vendor $vendor: $name"
      case ci"accept" => "accept"
      case _ => "other"
    println(kind(CIString("Content-Type")))
    println(kind(CIString("CONTENT-length")))
    println(kind(CIString("X-Acme-Trace")))
    println(kind(CIString("ACCEPT")))
    println(kind(CIString("Host")))
    val ci"$scheme://$host" = CIString("HTTPS://Example.org"): @unchecked
    println(s"$scheme $host ${host == ci"example.ORG"}")

    println("-- cats")
    println(CIString("a") === CIString("A"))
    println(CIString("a") =!= CIString("b"))
    println(cats.Order[CIString].compare(CIString("a"), CIString("B")) < 0)
