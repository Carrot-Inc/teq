// A member the JDK declares abstract is implemented, not overridden, by a subclass: no `override` asked
// (dotty's `ClassfileParser` gives the Java method `Deferred`; master asked for one on the JVM).
class Ones extends java.io.InputStream:
  var left = 3
  def read(): Int = if left == 0 then -1 else { left -= 1; 1 }
class Count extends java.io.OutputStream:
  var n = 0
  def write(b: Int): Unit = n += 1
class Collect extends java.io.Writer:
  val sb = new StringBuilder
  def write(cbuf: Array[Char], off: Int, len: Int): Unit =
    var i = off
    while i < off + len do { sb.append(cbuf(i)); i += 1 }
  def flush(): Unit = sb.append('|')
  def close(): Unit = sb.append('.')
@main def run(): Unit =
  val in = new Ones
  println(Iterator.continually(in.read()).takeWhile(_ != -1).sum)
  val anon = new java.io.InputStream { def read(): Int = 7 }
  println(anon.read())
  val out = new Count
  out.write(1)
  out.write(Array[Byte](1, 2, 3))
  println(out.n)
  val w = new Collect
  w.write("ab")
  w.flush()
  w.write('c'.toInt)
  w.close()
  println(w.sb)
