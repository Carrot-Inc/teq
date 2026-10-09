// A statement that reads a local object the block defines after it: the object is made on its
// first read, as scalac's lazy local module, here before its definition is reached, directly and
// through a local inline method whose import of the object's member is checked as written
// (`import source.k` between the call and `g`). teq reads the object's binding before the block
// declares it on all three targets (a temporal dead zone on JavaScript, "not in scope" from the
// JVM backend and the interpreter), in plain code too. scalac prints the lines of the .expected
// file.
object source { val k = 1 }
import source.k

@main def run(): Unit =
  println(g())
  println(source.k)
  object source { val k = 2 }
  import source.k
  inline def g(): Int = k
  println(g())
