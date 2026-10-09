// A cast evaluates its operand once, whatever it does: a failing cast, an unboxing of a value
// whose type is `Null` (`Erasure.Boxing.constant` keeps an operand with effects), a cast its
// erasure makes redundant, a cast to `Unit` and one to `Nothing`; a failing cast stops what
// follows it.
class A
class B

var n = 0
def value: Any = { n += 1; new A }
def nothing: Null = { n += 1; null }
def same: A = { n += 1; new A }

def fails(name: String)(f: => Any): Unit =
  try
    f
    println(name + " passed")
  catch case _: ClassCastException => println(name + " CCE")

@main def run(): Unit =
  fails("failing") { value.asInstanceOf[B]; n += 10 }
  println(n)
  println(nothing.asInstanceOf[Int])
  println(n)
  same.asInstanceOf[A]
  println(n)
  println(value.asInstanceOf[Unit] == ())
  println(n)
  fails("Nothing") { value.asInstanceOf[Nothing] }
  println(n)
  fails("widened") { val x: Any = value.asInstanceOf[B]; n += 10; x }
  println(n)
