// Aliases of the primitive classes and of String, and annotated primitives, in the places the typer compares types by
// identity: arithmetic and its widening, comparisons, overloads, string concatenation and interpolation, printing,
// pattern matches, collections and defaults. Under `--semantic-types` each of them is a retained form over its
// canonical type (tests/semanticdb/TABLE.md, section 6): the identity run holds the output alike.

type UserId = Long
type Score = Double
type Ratio = Float
type Count = Int
type Letter = Char
type Flag = Boolean
type Name = String
type Small = Byte

object Overloads:
  def pick(n: Count): String = "int"
  def pick(n: Long): String = "long"
  def wide(d: Score): Score = d * 2

case class Player(id: UserId, name: Name, score: Score = 1.5, letter: Letter = 'a')

def describe(x: Any): String = x match
  case n: Count => s"count $n"
  case d: Score => s"score $d"
  case s: Name => s"name $s"
  case _ => "other"

@main def run(): Unit =
  val id: UserId = 41
  val next = id + 1
  val c: Count = 3
  val r: Ratio = 2.25
  val s: Score = c + 0.5
  val l: Letter = 'z'
  val f: Flag = c > 2 && r < 3
  val n: Name = "ann"
  val b: Small = 7
  val annotated: Int @deprecated = 9
  println(next)
  println(c + r)
  println(s / 2)
  println(l + 1)
  println(f)
  println(n + c)
  println("x" + n + l)
  println(s"$n-$c-$r")
  println(r)
  println(b + b)
  println(annotated * 2)
  println(Overloads.pick(c))
  println(Overloads.pick(3))
  println(Overloads.wide(c) + 0.25)
  println(describe(c))
  println(describe(s))
  println(describe(n))
  val ids: List[UserId] = List(1, 2, 3)
  println(ids.sum + id)
  val counts = List[Count](1, 2).map(_ * r)
  println(counts)
  val p = Player(7, "bo")
  println(p.copy(score = p.score + c))
  println(n.length + n.toUpperCase)
  println(if f then l else 'n')
  val m: Map[Name, Count] = Map("a" -> 1)
  println(m.getOrElse(n, c) + 1L)
