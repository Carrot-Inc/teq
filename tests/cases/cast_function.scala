// A cast to a function type is the function type's test (the JVM's `checkcast` of `scala.Function1`;
// outside the JVM a function of any arity, tests/cases/js_cast_unchecked), a context function type
// and a polymorphic one being the function type of their arity
// (`TypeErasure.apply`), and `f.asInstanceOf[() => Int]()` applies the cast's value to the empty
// argument list (dotty's `Apply` of the `TypeApply`), which calls the function.
trait F extends (() => Int)

var calls = 0
given Int = 2

@main def run(): Unit =
  val f: Any = () => { calls += 1; 7 }
  println(f.asInstanceOf[() => Int]())
  println(calls)
  f.asInstanceOf[() => Int]()
  println(calls)
  val g: Any = (x: Int) => x + 1
  println(g.asInstanceOf[Int => Int](1))
  val h: Any = new F { def apply(): Int = 5 }
  println(h.isInstanceOf[F])
  println(h.asInstanceOf[F]())
  val p: Any = [T] => (t: T) => t
  println(p.asInstanceOf[[T] => T => T](3))
  try
    ("x": Any).asInstanceOf[[T] => T => T]
    println("poly passed")
  catch case _: ClassCastException => println("poly CCE")
  println(g.asInstanceOf[Int ?=> Int])
  val c: Any = (n: Int) ?=> n + 1
  println(c.asInstanceOf[Int => Int](3))
