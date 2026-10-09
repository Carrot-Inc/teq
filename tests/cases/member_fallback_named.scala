// An extension takes a member call whose first argument list does not fit the member: a named
// argument of no parameter of the member, or of the right name and another type, with the
// extension's defaults filling the rest, as scalac's `tryInsertImplicitOnQualifier` has it; a
// function literal of no parameters whose result fits the member's keeps the member.
extension (p: (Int, Int)) def apply(n: String = "default"): String = n
extension (p: (Int, String))
  def take(s: String = "D", n: Int = 2): String = s * n
  def splitAt(n: String, s: String = "D"): String = n + s
trait Printable:
  def pprint(v: () => String): Unit = println(v())
extension (ctx: Printable) def pprint(f: () => Int): Unit = ctx.pprint(() => "int " + f())
@main def run(): Unit =
  println((1, 2).apply(n = "s"))
  println((1, "x").take(n = 3, s = "a"))
  println((1, "x").take(s = "b"))
  println((1, "x").splitAt(n = "c"))
  println((1, "x").take(1))
  val printable = new Printable {}
  printable.pprint(() => "text")
