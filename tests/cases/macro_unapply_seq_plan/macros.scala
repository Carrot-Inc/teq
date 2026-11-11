import scala.quoted.*

// A macro's sequence patterns are read by the members the pattern's plan selects on the extractor's result
// (PatternMatcher.unapplySeqPlan): `lengthCompare` for the count, `apply` for each element, `drop` for the rest
// after some, `toSeq` for a rest with nothing before it; a library's wrapper and a result that is no collection.
final class Digits(s: String):
  def isEmpty: Boolean = false
  def get: Digits = this
  def lengthCompare(len: Int): Int = s.length.compare(len)
  def apply(i: Int): Int = s(i) - '0'
  def drop(n: Int): Seq[Int] = s.drop(n).map(_ - '0').toList
  def toSeq: Seq[Int] = s.map(_ - '0').toList

object Digits:
  def unapplySeq(s: String): Digits = new Digits(s)

// A repeated `apply` declared first is no element's selector; every element is taken before any is matched, and
// the rest only once they all have matched.
final class Traced(xs: List[Int], log: StringBuilder):
  def isEmpty: Boolean = false
  def get: Traced = this
  def apply(is: Int*): Seq[Int] = is.map(xs)
  def lengthCompare(len: Int): Int = { log ++= s"lc($len) "; xs.length.compare(len) }
  def apply(i: Int): Int = { log ++= s"ap($i) "; xs(i) }
  def drop(n: Int): Seq[Int] = { log ++= s"drop($n) "; xs.drop(n) }
  def toSeq: Seq[Int] = xs

object Traced:
  def unapplySeq(t: (List[Int], StringBuilder)): Traced = new Traced(t._1, t._2)

object M:
  inline def shapes(): String = ${ shapesImpl }

  def shapesImpl(using Quotes): Expr[String] =
    def digits(s: String): String = s match
      case Digits(a, b) => s"two $a $b"
      case Digits(a, rest*) => s"first $a then $rest"
      case Digits() => "none"
    def list(xs: List[Int]): String = xs match
      case List(a, b) => s"pair $a $b"
      case List(a, rest*) => s"head $a rest $rest"
      case List() => "empty"
    val all = "678" match
      case Digits(ds*) => ds.toString
    def traced(xs: List[Int]): String =
      val log = new StringBuilder
      val shape = (xs, log) match
        case Traced(9, b) => s"nine then $b"
        case Traced(0, rest*) => s"zero then $rest"
        case Traced(a, b) => s"two $a $b"
        case _ => "other"
      s"$shape: ${log.toString.trim}"
    Expr(List(digits("12"), digits("345"), digits(""), all, list(List(1, 2)), list(List(3, 4, 5)), list(Nil), traced(List(1, 2)), traced(List(0, 5))).mkString("\n"))
