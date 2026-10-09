// A cast of an inline body is decided where the body expands, with the call's type arguments, as
// dotty's erasure decides the casts of the inlined code: a cast to the type parameter checks the
// class it stands for, discarded, used, in a nested or transparent expansion; to `Int` it
// unboxes; after a test of the same value passed it cannot fail.
class A
class B

inline def cast[X](x: Any): X = x.asInstanceOf[X]
inline def fixed(x: Any): B = x.asInstanceOf[B]
inline def outer[U](x: Any): U = cast[U](x)
transparent inline def through[U](x: Any) = cast[U](x)
inline def guarded[X](v: Any): Option[X] = if v.isInstanceOf[X] then Some(v.asInstanceOf[X]) else None

def attempt(name: String)(f: => Any): Unit =
  try
    f
    println(name + " passed")
  catch case e: ClassCastException => println(name + ": " + e.getMessage.takeWhile(_ != '(').trim)

@main def run(): Unit =
  attempt("inline, discarded") { cast[B](new A); () }
  attempt("inline, used") { val b = cast[B](new A); b == null }
  attempt("inline of B") { val b = cast[B](new B); b == null }
  attempt("inline, fixed") { fixed(new A); () }
  attempt("nested") { outer[B](new A); () }
  attempt("transparent") { through[B](new A); () }
  attempt("inline to Int") { cast[Int]("text") + 1 }
  println(cast[Int](null) + 1)
  println(guarded[String]("text"))
  println(guarded[Int]("text"))
  println(guarded[Int](3))
  println(guarded[B](new B).isDefined)
