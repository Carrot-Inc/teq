package cab

import caa.*

def attempt(label: String)(body: => Any): Unit =
  try { body; println(label + " ok") }
  catch case _: ClassCastException => println(label + " CCE")
@main def run(): Unit =
  attempt("primitive") { Lib.primitive(Array(1)) }
  attempt("primitive-inline") { Lib.primitiveInline(Array(1)) }
  attempt("generic") { Lib.generic[Any]("text") }
  attempt("generic-inline") { Lib.genericInline[Any]("text") }
