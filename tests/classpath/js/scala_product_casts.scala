// jars: fixtures
// targets: js interp jvm
// The casts of a library scalac compiled (tests/tasty/src/casts.scala), read from its TASTy: an
// inline body's cast to a fixed class or to the type parameter is decided where the program
// expands it, and checked whether its result is used or discarded; an ordinary method's
// cast is checked in its own body.
import fix.casts.*

def attempt(name: String)(f: => Any): Unit =
  try
    f
    println(name + " passed")
  catch case _: ClassCastException => println(name + " CCE")

@main def run(): Unit =
  attempt("inline, discarded") { CastLib.checked(new CastA); () }
  attempt("inline, used") { val b = CastLib.checked(new CastA); b == null }
  attempt("inline of CastB") { val b = CastLib.checked(new CastB); b == null }
  attempt("generic, discarded") { CastLib.generic[CastB](new CastA); () }
  attempt("generic, used") { val b = CastLib.generic[CastB](new CastA); b == null }
  attempt("generic to Int") { CastLib.generic[Int]("text") + 1 }
  println(CastLib.generic[Int](null) + 1)
  attempt("ordinary") { CastLib.ordinary(new CastA); () }
