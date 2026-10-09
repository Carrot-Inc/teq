package lib.internal

trait Pretty[A]:
  def pretty(a: A): String

case class Version(major: Int, minor: Int)

object Strings:
  def exclaim(s: String): String = s + "!"

  val separator: String = ", "

  extension (s: String) def twice: String = s + s

  given Pretty[Version] with
    def pretty(v: Version): String = s"v${v.major}.${v.minor}"

object Numbers:
  def double(n: Int): Int = n * 2
