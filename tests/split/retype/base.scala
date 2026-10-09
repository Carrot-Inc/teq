package retype

/** What the macros' runs construct and read: a class with an initialiser and a lazy val, and a
  * top-level val. */
final class Tally(text: String):
  val words: List[String] = text.split(" ").toList.filter(_.nonEmpty)
  lazy val longest: Int = words.map(_.length).foldLeft(0)((a, b) => if a > b then a else b)
  def size: Int = words.length

val separator: String = "/"

inline def keptBy(word: String): Boolean = Holder.rule.keeps(word)
