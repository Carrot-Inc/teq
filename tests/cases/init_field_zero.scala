//> using platform jvm
// A field read before its initialiser ran holds its type's zero (`0`, `false`, `0.0`) or `null`, as
// on the JVM, in the interpreter too; on JavaScript it is `undefined` (COMPATIBILITY, "Known
// differences"), so this runs on the JVM's platform alone. An enum constructor reading a field
// of its companion, which makes the values first, reads it so.
enum E:
  case A
  val n: Int = E.seed
object E:
  val seed: Int = 7
  final val fixed = 8
  def computed: Int = 9
object A:
  val x: Int = B.y + 1
object B:
  val y: Int = A.x + 1
  val s: String = "s"
abstract class P:
  val n: Long
  val seen = n
class Q extends P:
  val n = 5L
class R:
  val flag: Boolean = early
  def early = late
  val late = true
  val c: Char = ch
  def ch = cc
  val cc = 'x'
  val d: Double = dd
  def dd = ddd
  val ddd = 1.5
@main def run(): Unit =
  println(A.x)
  println(B.y)
  println((new Q).seen)
  val r = new R
  println(r.flag)
  println(r.c.toInt)
  println(r.d)
  println(E.A.n)
  println(E.seed)
  println((E.fixed, E.computed))
