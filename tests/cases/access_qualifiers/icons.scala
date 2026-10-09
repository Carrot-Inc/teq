package ui.facade

object IconSet:
  private[facade] def mkIcon(name: String): String = s"<svg $name>"
  private[ui] val prefix: String = "icon-"
  protected[facade] def shade(n: Int): Int = n * 10
  private[IconSet] def secret: String = "s3cret"
  private[this] val hidden: Int = 1
  def reveal: String = secret + hidden

  object Nested:
    def viaOuter: String = secret + "/" + IconSet.secret

class Badge(private[facade] val level: Int, private[ui] var label: String):
  private[facade] def bump: Badge = Badge(level + 1, label)

class Token private[ui] (val raw: String)

object Gallery:
  def all: List[String] = List("a", "b").map(IconSet.mkIcon)
  def describe(b: Badge): String = s"${b.label}:${b.bump.level}:${IconSet.shade(2)}"
