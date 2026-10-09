// Extractor patterns on stable identifiers: a val holding a Regex (unapplySeq), a val or
// a lazy val with an unapply, and a path through objects and vals.
class Between(lo: Int, hi: Int):
  def unapply(n: Int): Option[Int] = if n >= lo && n <= hi then Some(n - lo) else None

class Words:
  def unapplySeq(s: String): Option[Seq[String]] = if s.isEmpty then None else Some(s.split(" ").toSeq)

object Rules:
  val small = new Between(0, 9)
  object nested:
    val words = new Words
    lazy val big = new Between(100, 200)

object Main:
  private val keyValue = "(\\w+)=(\\w+)".r
  private val number = "n(\\d+)".r
  private val date = "(\\d{4})-(\\d{2})-(\\d{2})".r

  def classify(s: String): String = s match
    case keyValue(k, v) => s"pair $k $v"
    case number(n) => s"number ${n.toInt + 1}"
    case date(y, m, _) => s"date $y/$m"
    case _ => "other"

  def size(n: Int): String = n match
    case Rules.small(offset) => s"small+$offset"
    case Rules.nested.big(offset) => s"big+$offset"
    case _ => "middle"

  def sentence(s: String): String =
    val words = Rules.nested.words
    s match
      case words(one) => s"one: $one"
      case words(first, second) => s"two: $first, $second"
      case words(first, rest*) => s"many: $first +${rest.length}"
      case _ => "none"

  def main(args: Array[String]): Unit =
    List("a=b", "n41", "2024-05-06", "zzz").map(classify).foreach(println)
    List(3, 50, 150).map(size).foreach(println)
    List("hi", "hi there", "one two three four", "").map(sentence).foreach(println)
    val local = "x(\\d)".r
    val found = List("x1", "y2", "x3").collect { case local(d) => d.toInt }
    println(found)
    var switching = local
    println("x7" match { case switching(d) => d; case _ => "-" })
    val all = "(a+)".r
    println("caab" match { case all(a) => a; case _ => "no" })
