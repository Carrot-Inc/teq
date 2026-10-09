//> using dep com.lihaoyi::sourcecode:0.4.2
// jars: sourcecode
package demo.desugar

import scala.collection.mutable.ArrayBuffer

def site(tag: String)(using n: sourcecode.Name, f: sourcecode.FullName, l: sourcecode.Line): String =
  s"$tag: ${n.value} | ${f.value}"

val log = ArrayBuffer[String]()
def note[A](tag: String, a: A): A =
  log += tag
  a

class Widget:
  def withLocals: String =
    val (first, second) = (site("tuple"), 2)
    val (third, fourth) = (site("tuple3"), site("tuple4"))
    val (a, b) = { val t = (site("block"), 1); t }
    val (c, d): (String, Int) = (site("typed"), 2)
    val Some(e) = Some(site("some")): @unchecked
    val (f, _) = (site("wild"), 3)
    val (g: String, h) = (site("typedBinder"), 3)
    val (i, _) = (site("wildUnchecked"), 3): @unchecked
    val (j, (k, _)) = (site("nested"), (1, 2))
    val ((l, _), _) = ((site("nestedSingle"), 1), 2)
    val q = site("plain")
    lazy val (m, n) = (site("lazy"), 1)
    List(first, third, fourth, a, c, e, f, g, i, j, l, q, m).mkString("\n")

@main def main(): Unit =
  println(Widget().withLocals)
  var (x, y) = (note("x", 1), note("y", "s"))
  x += 1
  y = y + "!"
  println(s"$x $y $log")
  val (p, r) = (note("p", 1.5), note("r", 'c'))
  val (s: Long, t) = (2, 3)
  println(s"$p $r $s $t $log")
  val (u, v) = (if x > 1 then (1, 2) else (3, 4))
  println(u + v)
