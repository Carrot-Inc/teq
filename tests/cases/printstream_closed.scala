// After close a print stream writes nothing and is in error (checkError); close itself sets no error.
import java.io.*
object Probe:
  def main(args: Array[String]): Unit =
    val b = new ByteArrayOutputStream
    val p = new PrintStream(b)
    p.print("before"); p.close()
    println("closed-error=" + p.checkError())
    p.print("after"); p.write(65); p.flush()
    println("bytes=" + b.toString("UTF-8"))
    println("write-error=" + p.checkError())
