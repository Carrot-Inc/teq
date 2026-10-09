// Expansions of one inline method that differ only in what the call site gave (the literal of
// `constValue`, the given of `summonInline`, the type tested, the arguments) share one function
// in the JavaScript output; a leaf inside a lambda is still evaluated where it stood, and an
// expansion that returns from its method or assigns a local of its site stays where it is.
import scala.compiletime.{constValue, summonInline}

trait Show[A]:
  def show(a: A): String

object Trace:
  var lines: List[String] = Nil
  def note(s: String): Unit = lines = s :: lines

object IntShow extends Show[Int]:
  Trace.note("IntShow ready")
  def show(a: Int): String = "int " + a

object Show:
  given Show[Int] = IntShow
  given Show[String] with
    def show(a: String): String = "str " + a

object Fields:
  inline def describe[L <: String, A](value: Any, inline tag: String): String =
    val label = constValue[L]
    val show = () => summonInline[Show[A]]
    Trace.note("describe " + label)
    val kind = if value.isInstanceOf[A] then "match" else "other"
    val rendered = value match
      case a: A => show().show(a)
      case _ => "?"
    label + "=" + rendered + " (" + kind + ", " + tag + ")"

  inline def firstOr(xs: List[Int], inline fallback: Int): Int =
    val size = xs.length
    Trace.note("firstOr of " + size)
    val head = xs.headOption match
      case Some(h) if h > 0 => h * 10
      case Some(h) => h
      case None => fallback
    head + size * 0

object Main:
  def find(xs: List[Int]): Int =
    val a = Fields.firstOr(xs, return -1)
    a + 1

  def safe(xs: List[Int]): Int =
    val rest = xs.drop(1)
    Fields.firstOr(xs, 7) + Fields.firstOr(rest, 8)

  def count(xs: List[Int]): Int =
    var total = 0
    inline def add(inline n: Int, label: String): Unit =
      val before = total
      total = total + n
      Trace.note(label + " " + before + " -> " + total)
    add(xs.length, "first")
    add(xs.sum, "second")
    total

  def main(args: Array[String]): Unit =
    println(Fields.describe["a", Int](1: Any, "x"))
    println(Fields.describe["b", String]("s": Any, "y"))
    println(Fields.describe["c", Int]("no": Any, "z"))
    println(Fields.describe["d", String](2: Any, "w"))
    println(find(List(3, 4)))
    println(find(Nil))
    println(safe(List(0, 5)))
    println(safe(Nil))
    println(count(List(1, 2, 3)))
    Trace.lines.reverse.foreach(println)
