// The emitter's own temporaries and the parameters of a def written as a loop are bindings no
// source names: a local named like the temporary a match keeps its scrutinee in, or like the
// loop's parameter, is named apart from it.
package temps

def id(x: Any): Any = x

def scrutinee(x: Any): Int = id(x) match
  case n: Int =>
    val `$s1` = 10
    n + `$s1`
  case _ => 0

def n(): Int = 7

def loop(n: Int): Int =
  val `n$1` = 10
  if n == 0 then temps.n() + `n$1`
  else loop(n - 1)

@main def main(): Unit =
  println(scrutinee(4))
  println(loop(1))
