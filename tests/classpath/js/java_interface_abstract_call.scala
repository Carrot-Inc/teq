// std: scala-library
// A call of an abstract Java interface method on JavaScript runs the receiver's implementation: no JDK code is
// needed, whatever the std's platform layer gives the interface.
def invoke(r: java.lang.Readable): Int = r.read(null)

class R extends java.lang.Readable:
  override def read(buffer: java.nio.CharBuffer): Int = 42

@main def run(): Unit = println(invoke(new R))
