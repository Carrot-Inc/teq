// A structural given over a constructor call is dotty's structural instance (`Parsers.givenDef`,
// 4610-4623): without parameters a module whose class extends the call, its body's members its
// own (`O.t.extra`) and published before its body runs; with parameters a class beside a given
// def, locally as well, capturing the block's values.
trait Parent(val n: Int):
  val self: Parent
class Base(val m: Int)
trait Show[A] { def s: String }
def helper: Parent = t
given t: Parent(1):
  val self: Parent = helper
  def extra = "ok" + n
object O:
  given b: Base(2):
    def extra = "b" + m
  given QS: Show[Int]() with
    def s = "qs"
@main def run(): Unit =
  println(t.self eq t)
  println(t.extra + O.b.extra + summon[Base](using O.b).m)
  println(summon[Show[Int]](using O.QS).s)
  val pre = "p"
  var made = 0
  given [A](using n: Int): Show[A] with
    made += 1
    def s = pre + n
  given Int = 3
  println(summon[Show[String]].s)
  println(summon[Show[Char]].s + made)
  given showList[A](using e: Show[A]): Show[List[A]] with
    def s = "L" + e.s
  println(summon[Show[List[Boolean]]].s)
  given u: Parent(7):
    val self: Parent = this
    def tag = "u" + n + pre
  println(u.tag + summon[Parent].n)
