import Tailwind.*

object Tailwind:
  final class Tw(val value: String):
    def ++(other: Tw): Tw = Tw(value + " " + other.value)
    override def toString: String = "Tw(" + value + ")"

  object Tw:
    val empty: Tw = Tw("")

  extension (sc: StringContext)
    def tw(args: Any*): Tw = Tw(sc.s(args*))

  def inside(n: Int): Tw = tw"inside-$n"

extension (sc: StringContext)
  // Shows what the compiler hands over: the parts as written and the evaluated arguments.
  def show(args: Any*): String =
    var out = ""
    var i = 0
    while i < sc.parts.length do
      out = out + "[" + sc.parts(i) + "]"
      if i < args.length then out = out + "<" + args(i).toString + ">"
      i += 1
    out
  def count(args: Any*): Int = sc.parts.length * 100 + args.length
  def sum(args: Int*): Int =
    var total = 0
    args.foreach(a => total += a)
    total + sc.parts.size
  def viaS(args: Any*): String = sc.s(args*)
  def viaRaw(args: Any*): String = sc.raw(args*)
  def first[A](args: A*): A = args(0)
  def sized(args: Any*)(using scale: Int): Int = sc.parts.toList.map(_.length).sum * scale

final case class Point(x: Int, y: Int)

def styled(text: String, style: Tw = tw"text-sm"): String = style.value + ":" + text

def pick(wide: Boolean): Tw = if wide then tw"w-full" else Tw.empty

@main def main(): Unit =
  val n = 3
  val name = "x"
  val p = Point(1, 2)

  println(tw"text-sm font-bold")
  println(tw"w-${n} px")
  println(tw"w-$n" ++ tw"h-${n * 2}")
  println(inside(7))
  println(styled("a"))
  println(styled("b", tw"text-lg"))
  println(pick(true))
  println(pick(false))

  println(show"plain")
  println(show"")
  println(show"$n")
  println(show"$n$name")
  println(show"a $n b ${name + "!"} c")
  println(show"${p.x + p.y} and $p")
  println(show"tab\tnewline\nquote\"backslash\\ dollar$$ $name end")
  println(show"""triple "quoted" \t $n""")
  println(show"nested ${show"inner $n"} done")
  println(show"this ${if n > 2 then "big" else "small"} one")

  println(count"a $n b $name c")
  println(sum"$n + ${n * 2} + ${10}")
  println(first"$name and $name".length)
  println(first"${n + 1}" + 1)
  given Int = 10
  println(sized"ab${n}cde")

  println(viaS"tab\tquote\" dollar$$ $n")
  println(viaRaw"tab\tquote\" dollar$$ $n")
  println(s"tab\tquote\" dollar$$ $n")
  println(raw"tab\tquote\" dollar$$ $n")
  println(s"""triple\tquoted $n""")
  println(raw"""triple\tquoted $n""")

  val sc = StringContext("a", "b\\n", "c")
  println(sc.parts.length)
  println(sc.parts(1))
  println(sc.s(1, 2))
  println(sc.raw(1, 2))
  println(StringContext.processEscapes("x\\ty"))
  println(show"a${1}b".length)
