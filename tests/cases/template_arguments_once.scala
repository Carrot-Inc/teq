// A strict argument of a library method is evaluated once, in order, before the method runs,
// whatever its JavaScript template does with it: repeats it, tests it first, or leaves it out.
var n = 0
def next(): Any = { n += 1; n }
def int(i: Int): Int = { n += 1; i }
def str(s: String): String = { n += 1; s }
def show(label: String, v: Any): Unit = println(s"$label $v $n")

@main def run(): Unit =
  show("hashCode", java.util.Objects.hashCode(next()))
  show("toString", java.util.Objects.toString(next()))
  show("signum", java.lang.Integer.signum(int(-5)))
  show("trailing", java.lang.Integer.numberOfTrailingZeros(int(8)))
  show("compare", java.lang.Integer.compare(int(1), int(2)))
  show("dropRight", str("abcd").dropRight(1))
  show("stripSuffix", str("abcd").stripSuffix(str("cd")))
  show("stripPrefix", str("abcd").stripPrefix("ab"))
  show("capitalize", str("abc").capitalize)
  val $t0 = "kept"
  val $t1 = "cd"
  show("named", str("abcd").stripSuffix($t1) + $t0)
  val order = new StringBuilder
  def at(c: Char, i: Int): Int = { order += c; i }
  show("order", java.lang.Integer.compare(at('a', 3), at('b', 2)))
  println(order)
  // An effect that runs the first time alone (a lazy val, an object's initialiser) comes in its
  // place among the arguments' effects.
  lazy val lazyOne: Int = { println("lazy"); 1 }
  def two(): Int = { println("two"); 2 }
  show("lazy first", java.lang.Integer.compare(lazyOne, two()))
  show("lazy again", java.lang.Integer.compare(lazyOne, two()))
  show("object first", java.lang.Integer.compare(Init.value, two()))
  show("sum", java.lang.Integer.compare(two() + 1, lazyOne))

object Init:
  println("init")
  val value: Int = 5
