// A type test or pattern against a type parameter checks the erasure of its bound (dotty's `TypeTestsCasts`),
// positive and negative witnesses of a reference bound, a primitive bound and an applied one. A `String` passes a test
// of `CharSequence` and of `String`'s other interfaces on every target: master's JVM folded a test of the std's
// stand-in for a JDK interface to false (no class of the program makes one), and JavaScript took a string for no
// instance of them, where Scala.js tests them as `String`'s.
def test[T <: CharSequence](x: Any): Boolean = x.isInstanceOf[T @unchecked]
def pat[T <: CharSequence](x: Any): Boolean = x match { case _: T @unchecked => true; case _ => false }
def itest[T <: Int](x: Any): Boolean = x.isInstanceOf[T @unchecked]
def ltest[T <: List[Int]](x: Any): Boolean = x.isInstanceOf[T @unchecked]
@main def run() =
  println(test[String](1))
  println(pat[String](1))
  println(test[String]("x"))
  println(pat[String]("x"))
  println(test[StringBuilder](new StringBuilder("y")))
  println(itest[Int](1))
  println(itest[Int]("x"))
  println(ltest[List[Int]](List(1)))
  println(ltest[List[Int]](Vector(1)))
  val a: Any = "x"
  println(a.isInstanceOf[CharSequence])
  println(a.isInstanceOf[Comparable[?]])
  println(a.isInstanceOf[java.io.Serializable])
  a match
    case c: CharSequence => println(c.length)
    case _ => println("no")
