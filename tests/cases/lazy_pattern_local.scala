// A local `lazy val` of a pattern is dotty's `makePatDef` for a lazy definition: a lazy holder of
// the match and a lazy val per variable reading it, so that the right-hand side and the match run
// once, at the first read of any variable, and never when none is read.
case class P(x: Int, y: String)
object Ext:
  def unapply(n: Int): Option[(Int, Int)] = { println("unapply"); Some((n / 10, n % 10)) }

def never(): Unit =
  lazy val (a, b) = { println("never"); (1, 2) }
  println("skipped")

@main def run(): Unit =
  lazy val (a, b) = { println("init"); (1, 2) }
  println("before")
  println(a)
  println(b)
  println(a + b)
  never()
  lazy val Some(x) = { println("some"); Option(5) }: @unchecked
  println("x next")
  println(x)
  lazy val (m, (n, o)) = { println("nested"); (1, (2, 3)) }
  println("nested next")
  println(o + m)
  lazy val List(h, t*) = { println("list"); List(1, 2, 3) }: @unchecked
  println("list next")
  println(t)
  val big = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)
  lazy val (a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23) =
    { println("big"); big }
  println("big next")
  println(a23 + a1 + a12)
  lazy val (p: Int, q) = { println("typed"); (1, "q") }
  println(q + p)
  lazy val Ext(d, u) = { println("rhs"); 42 }: @unchecked
  println("ext next")
  println(d)
  println(u)
  lazy val P(px, py) = P(3, "three")
  val f = () => px + py.length
  println(f())
  lazy val Some(bad) = (None: Option[Int]): @unchecked
  println("bad not forced")
  try println(bad) catch case e: MatchError => println("MatchError")
  var count = 0
  def make(): Int =
    lazy val (m, n) = { count += 1; (count, count * 2) }
    m + n
  println(make() + make())
