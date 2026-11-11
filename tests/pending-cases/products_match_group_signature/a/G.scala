package gp

object G:
  def m(s: String): String =
    val r = "a(b)".r
    r.findFirstMatchIn(s).get.group(1)
  def m3(s: String): String =
    val r = "a(b)".r
    val mm: scala.util.matching.Regex.Match = r.findFirstMatchIn(s).get
    mm.group(1)
