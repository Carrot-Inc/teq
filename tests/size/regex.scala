// The regular-expression runtime with a pattern built at run time, so that the translator from
// Java's syntax to the engine's is reached: matching, extraction, iteration, splitting and
// replacement.
object Patterns:
  private val parts = List("[a-z", "]+", "(\\d+)-(\\d+)", "\\s*,\\s*", "(?i)he(l+)o")
  def word: String = parts(0) + parts(1)
  def range: String = parts(2)
  def comma: String = parts(3)
  def hello: String = parts(4)

@main def run(): Unit =
  val text = "Hello there, 12-34 and 5-6, hELLo"
  val word = Patterns.word.r
  println(word.findAllIn(text).toList)
  println(word.findFirstIn(text))
  val range = Patterns.range.r
  for m <- range.findAllMatchIn(text) do println(s"${m.start} ${m.group(1)} ${m.group(2)}")
  text match
    case s if range.findPrefixOf(s).isDefined => println("prefix")
    case _ => println("no prefix")
  println(text.split(Patterns.comma).toList)
  println(Patterns.hello.r.replaceAllIn(text, m => m.group(1).length.toString))
  println(range.replaceFirstIn(text, "$2-$1"))
  println("12-34".matches(Patterns.range))
