// The classes an inline body makes copied with the classes they need: a local class that extends
// another the body never creates itself, the parent settled first with what it captures (`base`)
// and its kept calls expanded; a class nested in a local class, copied with it at each expansion
// and named apart, one reading its owner's member through the owner's `this` the copy of the
// owner holds; a class used through its parent's type alone. Two expansions of each, which
// the retype path names alike. scalac prints the lines of the .expected file.
inline def inc(x: Int): Int = x + 1
inline def make(k: Int): Int =
  val base = k * 2
  class Parent { def value: Int = inc(base) + 7 }
  class Child extends Parent { def twice = value * 2 }
  new Child().twice
inline def nested(k: Int): Int =
  class Outer { class Inner { def v = inc(k) }; def make = new Inner }
  new Outer().make.v
inline def owned(k: Int): Int =
  class Box(val value: Int):
    class Reader:
      def get: Int = value
    def get: Int = new Reader().get
  new Box(k).get
inline def typeOnly: String =
  class Shape { def name = "shape" }
  class Square extends Shape
  val s: Shape = new Square
  s.name
@main def run(): Unit =
  println(make(1))
  println(make(5))
  println(nested(3))
  println(nested(4))
  println(owned(7) + owned(8))
  println(typeOnly + " " + typeOnly)
