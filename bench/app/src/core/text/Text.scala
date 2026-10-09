package meridian.core.text

object Text:
  def slug(s: String): String =
    s.toLowerCase.map(c => if c.isLetterOrDigit then c else '-').split("-").filter(_.nonEmpty).mkString("-")

  def capitalise(s: String): String = if s.isEmpty then s else s.charAt(0).toUpper.toString + s.substring(1)

  def words(s: String): List[String] = s.split("\\s+").toList.filter(_.nonEmpty)

  def padLeft(s: String, width: Int, fill: Char = ' '): String =
    if s.length >= width then s else fill.toString * (width - s.length) + s

  def padRight(s: String, width: Int, fill: Char = ' '): String =
    if s.length >= width then s else s + fill.toString * (width - s.length)

  def truncate(s: String, max: Int): String = if s.length <= max then s else s.substring(0, Math.max(0, max - 1)) + "…"

  def plural(n: Long, singular: String, plural: String): String = if n == 1 then s"$n $singular" else s"$n $plural"

  def initials(s: String): String = words(s).map(_.charAt(0).toUpper).mkString

  def column(rows: List[List[String]]): String =
    if rows.isEmpty then ""
    else
      val widths = rows.map(_.map(_.length)).transpose.map(_.max)
      rows.map(r => r.zip(widths).map((cell, w) => padRight(cell, w)).mkString(" ").trim).mkString("\n")

/** A 64-bit FNV-1a hash over text, kept as a signed Long so both toolchains print it alike. */
object Checksum:
  private val Offset = -3750763034362895579L
  private val Prime = 1099511628211L

  def of(text: String): Long =
    var h = Offset
    var i = 0
    while i < text.length do
      h = (h ^ text.charAt(i).toLong) * Prime
      i += 1
    h

  final class Builder:
    private var hash = Offset
    private var count = 0
    def add(text: String): Builder =
      var i = 0
      while i < text.length do
        hash = (hash ^ text.charAt(i).toLong) * Prime
        i += 1
      hash = (hash ^ 10L) * Prime
      count += 1
      this
    def lines: Int = count
    def value: Long = hash
