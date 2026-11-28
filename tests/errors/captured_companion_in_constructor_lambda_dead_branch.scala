def read(n: Int): Int =
  class Counter(val x: Int):
    def this(s: String = "a") = this(n + s.length)
  class Box(val x: Int):
    def this() = this({
      if false then (() => new Counter().x)() else 1
    })
  new Box().x

object Probe:
  def main(args: Array[String]): Unit = println(read(2))

// A closure in a secondary constructor uses the default argument of another local class. The
// expression occurs in the untaken arm of a constant conditional.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
