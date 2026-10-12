// `new:` without parents against an expected type through a prefix, a trait or class nested in a
// class: its parent is that type, prefix and all, as dotty's `typedNew` takes the prototype whose
// `underlyingClassRef` is eligible.
class Outer(val base: Int):
  trait Inner:
    def value: Int
    def shifted: Int = value + base
  class Box:
    def label: String = "box"

@main def run(): Unit =
  val o = new Outer(10)
  val x: o.Inner = new:
    def value = 7
  println(x.value)
  println(x.shifted)
  val b: o.Box = new:
    override def label = "boxed"
  println(b.label)
