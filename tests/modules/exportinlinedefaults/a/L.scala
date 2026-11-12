package eid

// Inline members with inline defaults exported plainly, of a class value's members and of an object's. Each
// forwarder's getter is inline, its pickled body the member's getter on the forwarder's path (`Namer.addForwarder`,
// Namer.scala:1385-1387), and that getter an inline method of its own (an inline parameter's getter is inline,
// Desugar.scala:429), which a downstream over teq's products is to expand from its pickle (`Inlines.inlineCall`): no
// class holds a method of it.
class Box(val k: Int):
  inline def p(inline x: Int = 7): Int = x + k
  inline def q(a: Int)(inline b: Int = a + 1): Int = a * 10 + b + k
object Impl:
  inline def t(inline x: Int = 3): Int = x * 2
object Fw:
  val bx = Box(4)
  export bx.{p, q}
  export Impl.t
