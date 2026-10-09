//> using platform js
// A parameterised alias where an expected type's shape is read: a union alias takes a lambda
// through its function member, a literal and a widened number; a function alias types a
// lambda's parameters; a context-function alias wraps a value; a literal-union alias keeps a
// literal narrow.
type Maybe[+A] = A | Unit
type Handler[A] = (A, Int) => String
type Reader[A] = String ?=> A
type Mode = "all" | "any"
type Modes[A] = Maybe[A | Mode]

class Props:
  var onClick: Maybe[(Int, String, Boolean) => Unit] = ()
  var mode: Maybe[Mode] = ()
  var size: Maybe[Double] = ()
  var handler: Maybe[Handler[Int]] = ()
  var reader: Reader[Int] = 0
  var modes: Modes[Int] = ()

def run(r: Reader[Int]): Int = r(using "ctx")
def describe(m: Maybe[Double]): String = m match
  case d: Double => s"double $d"
  case _ => "none"
def whenSet[A](m: Maybe[A])(f: A => Unit): Unit = m match
  case () => println("none")
  case a => f(a.asInstanceOf[A])

@main def main(): Unit =
  val p = Props()
  p.onClick = (day, _, _) => println(s"clicked $day")
  whenSet(p.onClick)(f => f(7, "x", true))
  p.mode = "all"
  println(p.mode)
  p.size = 3
  println(describe(p.size))
  p.handler = (a, n) => s"$a/$n"
  whenSet(p.handler)(f => println(f(1, 2)))
  p.reader = summon[String].length
  println(run(p.reader))
  println(run(41 + 1))
  p.modes = "any"
  println(p.modes)
  p.modes = 5
  println(p.modes)
  val h: Handler[String] = (s, n) => s * n
  println(h("ab", 2))
  val direct: Maybe[Int => Int] = x => x * 2
  whenSet(direct)(f => println(f(21)))
  whenSet[Int](())(n => println(n))
