package eor

// Forwarders made where a lookup first needs them: a class exporting another class's forwarder
// declared after it, and a class exporting through a top-level forwarder of its file.
class C(val a: A):
  export a.x
  export a.inner.z
class A(val b: B):
  export b.{x, inner}
class B:
  val x = 7
  val inner = new Inner
class Inner:
  val z = 3

class Lib:
  val inner = new Inner
val lib = new Lib
class E:
  export inner.z
export lib.inner
