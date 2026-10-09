package cib

import cia.*

def attempt(name: String)(f: => Any): Unit =
  try
    f
    println(name + " passed")
  catch case _: ClassCastException => println(name + " CCE")

@main def run(): Unit =
  attempt("inline, discarded") { Lib.checked(new A); () }
  attempt("inline, used") { val b = Lib.checked(new A); b == null }
  attempt("inline of B") { val b = Lib.checked(new B); b == null }
  attempt("generic, discarded") { Lib.generic[B](new A); () }
  attempt("generic, used") { val b = Lib.generic[B](new A); b == null }
  attempt("generic to Int") { Lib.generic[Int]("text") + 1 }
  println(Lib.generic[Int](null) + 1)
  attempt("ordinary") { Lib.ordinary(new A); () }
