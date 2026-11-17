package opa

// Two objects whose class has an initialiser by its source and none by its pickle: a parent's argument is not
// read from a pickle, and a nested object is no initialisation there.
class Arg(x: Int)
object Argument extends Arg({ println("  parent's argument"); 1 })
class Sec(x: Int) { def this() = this({ println("  secondary's argument of the inner object"); 1 }) }
object Outer { object Inner extends Sec() }
