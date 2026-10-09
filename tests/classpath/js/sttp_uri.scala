// jars: scala-library sttp-model sttp-shared-core scala-java-time
//> using dep com.softwaremill.sttp.model::core:1.7.18
// sttp-model 1.7.18's `uri"..."` interpolator from its jar: literals, embedded values in the
// scheme, host, path, query and fragment, `Option` and sequence values, `?` and `&` handling,
// `Uri.show`, the parts of the parsed uri and the encoding of special characters.
package sttpuri

import sttp.model.Uri
import sttp.model.Uri.*
import sttp.model.UriInterpolator

object Main:
  def main(args: Array[String]): Unit =
    val host = "api.example.com"
    val id = 42
    val name = "rex fido"
    val tag: Option[String] = Some("big")
    val none: Option[String] = None
    val tags = List("a", "b")
    val port = 8080
    val q = "x&y=z"
    println(uri"https://$host:$port/pets/$id?name=$name&tag=$tag&missing=$none#frag".toString)
    println(uri"http://example.com/a/b?q=$q".toString)
    println(uri"http://example.com/users/${"john doe"}/ok".toString)
    println(uri"http://example.com/list?tags=$tags".toString)
    println(uri"http://example.com/p?${Map("k1" -> "v1", "k2" -> "v2")}".toString)
    println(uri"http://example.com/p?a=1&$q".toString)
    val base = uri"https://example.com/base"
    println(uri"$base/more?x=1".toString)
    val parsed = uri"https://user:pw@example.com:9090/a/b%20c?x=1&y=2&y=3#top"
    println(parsed.scheme.toString + " " + parsed.host + " " + parsed.port + " " + parsed.userInfo + " " + parsed.path + " " + parsed.querySegments + " " + parsed.fragment)
    println(parsed.params.getMulti("y").toString + " " + parsed.paramsSeq + " " + parsed.pathSegments.segments.map(_.v))
    println(uri"http://example.com".toString + " " + uri"http://example.com/".toString + " " + uri"http://example.com?".toString + " " + uri"/relative/path?a=b".toString)
    println(uri"http://example.com/a".addPath("b", "c d").addParam("k", "v w").fragment("f g").toString)
    println(Uri.parse("https://example.com:1234/x?y=1").map(_.toString).toString + " " + Uri.unsafeParse("http://h/p").host)
    println(uri"http://example.com/$id/$name".withWholePath("new/path").toString + " " + uri"http://example.com/x".scheme("ftp").toString)
    println(uri"${"https://dyn.example.com/v"}/one".toString + " " + uri"http://example.com/?empty=&x=y".toString)
