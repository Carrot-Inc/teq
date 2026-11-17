// The shapes the levels do not hold: a class of one simple name in two packages (the split output names an
// imported class otherwise than its own module does); a plain parameter named as a parent's val, which takes no
// val; an overloaded name beside the val; a trait's parameter, which its initialiser writes after the bodies
// above it; names with a dollar sign. scalac warns of a body's redefinition of a val parameter (silenced).
package a
class P { val v: String = "p"; def read: String = v }
class A(override val v: String) extends P
