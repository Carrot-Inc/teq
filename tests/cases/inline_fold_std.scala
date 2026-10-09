// Inline parameters whose arguments are pure expressions over the standard library: the
// compiler evaluates them at compile time and binds the parameter to the constant, so that
// the body sees a literal. Effects, exceptions and expensive computations stay as they are.
inline def twice(inline x: Int): Int = x + x
inline def shout(inline s: String): String = s + "!"
inline def pick(inline flag: Boolean, inline a: String, inline b: String): String = if flag then a else b
inline def label(inline n: Int): String = "n=" + n.toString

var counter = 0
def bump(): Int =
  counter += 1
  counter

@main def run(): Unit =
  println(twice("ab".length + 1))
  println(twice(List(1, 2, 3).sum))
  println(shout(s"${"a" * 3}-${List(4, 5).mkString("/")}"))
  println(pick("abc".startsWith("a"), "yes", "no"))
  println(label(Vector(1, 2, 3, 4).map(_ * 2).filter(_ > 2).size))
  println(label("x,y,z".split(",").length))
  println(twice(bump()))
  println(counter)
  println(twice((1 to 100000).sum))
  println(pick(math.sqrt(16.0) == 4.0, "four", "other"))
  println(label(Map("a" -> 1, "b" -> 2).values.sum))
  println(shout(Option(7).map(_ + 1).getOrElse(0).toString))
  println(twice(scala.util.Random(3).nextInt(1)))
