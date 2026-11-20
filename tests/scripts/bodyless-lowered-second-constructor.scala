// A creation through a second constructor in lowered code is refused before its arguments run, as the walker
// refuses it: the JVM initializes the created class's Java superclass at `new`, before them.
class Shelf() extends jlib.Registry:
  def this(label: String) = this()

def make(b: Boolean): Int =
  if b then
    new Shelf({ println("label"); "top" })
    1
  else 0

@main def run(): Unit =
  println(make(false))
  println(make(true))
