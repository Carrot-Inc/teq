// `&&` and `||` decided on the receiver's type after its typing as the operator's qualifier: a type parameter
// bounded by `Boolean` has `Boolean`'s short circuit (its members), a boxed `java.lang.Boolean` reaches it through
// Predef's view, and an operand typed alone (`getOrElse`'s result) is a `Boolean` without the operator's help.
var log = ""
def eff(s: String, v: Boolean): Boolean = { log += s; v }
def bounded[T <: Boolean](t: T): Boolean = t && eff("b", true)
inline val On = true
@main def run(): Unit =
  val jb: java.lang.Boolean = java.lang.Boolean.valueOf(false)
  println(jb && eff("X", true))
  println(jb || eff("y", false))
  println(bounded(false))
  println((On && false, On || eff("Z", true)))
  val m = Map(1 -> true)
  println(m.getOrElse(2, false) && eff("W", true))
  println(Option(true).exists(identity) && !Option(false).contains(true))
  println(log)
