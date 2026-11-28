def read(n: Int): Int =
  class Counter(val x: Int):
    def this(s: String = "a") = this(n + s.length)
  class Box(val x: Int):
    def this() = this(new Counter().x)
  new Box().x

object Probe:
  def main(args: Array[String]): Unit = println(read(2))

// One local class uses another's secondary constructor default before calling its own primary
// constructor.
// scalac 3.8.4 builds it and prints `3`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Box$16tjli5_112.<init>: the local Counter is not in scope of the generated method
// expect: 1 construct not supported by the JVM backend
