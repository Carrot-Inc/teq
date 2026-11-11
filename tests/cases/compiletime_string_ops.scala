import scala.compiletime.ops.string.*
val a: Substring["hamburger", 4, 8] = "urge"
val b: Matches["unhappy", "un.*"] = true
val c: CharAt["hello", 0] = 'h'
val d: Length["hello"] = 5
val e: "a" + "b" = "ab"
val f: Matches["\r", "(?d)."] = true
val g: Matches["\n", "(?d)."] = false
val h: Matches["\u0661", "(?U)\\d"] = true
val i: Matches["\u2003", "(?U)\\s"] = true
val j: Matches["\u0661", "\\d"] = false
val k: Matches["𝟘", "(?U)\\d"] = true
val l: Matches["\u0661", "(?U)[\\d]"] = true
val m: Matches["\u2003", "(?U)[\\s]"] = true
val n: Matches["\u2003", "(?U)[\\S]"] = false
@main def stringOps(): Unit =
  println(a)
  println(b)
  println(c)
  println(d)
  println(e)
  println(f)
  println(g)
  println(h)
  println(i)
  println(j)
  println(k)
  println(l)
  println(m)
  println(n)
