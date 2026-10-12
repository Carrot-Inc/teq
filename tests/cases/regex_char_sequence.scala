// A Regex reads any CharSequence, as scala-library's takes one: a StringBuilder, a view of another
// sequence (parser combinators match their input's `SubSequence`), and a match's `before` and
// `after` are sequences of the source.
final class Window(s: String, from: Int) extends CharSequence:
  def length: Int = s.length - from
  def charAt(i: Int): Char = s.charAt(from + i)
  def subSequence(start: Int, end: Int): CharSequence = s.substring(from + start, from + end)
  override def toString: String = s.substring(from)

@main def run(): Unit =
  val word = "[a-z]+".r
  val sb = new StringBuilder("abc 123 def")
  println(word.findFirstIn(sb))
  println(word.findAllIn(sb).toList)
  println(word.findPrefixOf(new Window("xy12zz", 0)))
  println(word.findPrefixMatchOf(new Window("12zz", 2)).map(_.end))
  println("[0-9]+".r.matches(new Window("ab42", 2)))
  val m = "[0-9]+".r.findFirstMatchIn(sb).get
  println(s"<${m.before}|${m.matched}|${m.after}>")
  println(m.before.length + m.after.length)
  println("\\s+".r.replaceAllIn(sb, "_"))
  println("\\s+".r.split(sb).toList)
  val Numbered = "[a-z]+ ([0-9]+) .*".r
  (sb: CharSequence) match
    case Numbered(n) => println(n)
    case _ => println("none")
