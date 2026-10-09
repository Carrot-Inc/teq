// A type test against a function type checks the arity.
type I2I = Int => Int
def fn(x: Any): String = x match
  case f: I2I @unchecked => "fn1"
  case f: Function2[?, ?, ?] => "fn2"
  case f: Function0[?] => "fn0"
  case _ => "other"

def add(a: Int, b: Int): Int = a + b

@main def run(): Unit =
  println(fn((x: Int) => x))
  println(fn((x: Int, y: Int) => x))
  println(fn(() => 1))
  println(fn(add))
  println(fn("x"))
  println(((x: Int, y: Int) => x).isInstanceOf[Function2[?, ?, ?]])
  println(((x: Int) => x).isInstanceOf[Function2[?, ?, ?]])
  val fs: List[Any] = List((x: Int) => x, (a: Int, b: Int) => a + b, 3)
  println(fs.collect { case f: Function1[?, ?] => "one" })
