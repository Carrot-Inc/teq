// An owner's forwarders are made where a lookup first needs them, whatever order the owners stand
// in: a class exporting another class's forwarder declared after it, a path through a top-level
// forwarder of the file, and a path through a member that is itself a forwarder (dotty's
// `Namer.processExports` of the owner a lookup completes).
class D(val c: C):
  export c.x as y
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

object O:
  export Later.w
object Later:
  val w = "w"

@main def run(): Unit =
  val c = new C(new A(new B))
  println(new D(c).y + c.x + c.z)
  println(new E().z)
  println(O.w)
