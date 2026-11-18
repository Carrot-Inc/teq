// A print stream's encoder keeps its state across prints: the first half of a surrogate pair waits for its second
// (no byte after the first print, the four after the second), and a UTF-16 stream writes its byte-order mark once.
import java.io.*
object Probe:
  def main(args: Array[String]): Unit =
    val b = new ByteArrayOutputStream
    val p = new PrintStream(b, false, "UTF-8")
    p.print('\uD83D'); p.flush()
    println("held=" + b.size())
    p.print('\uDE00'); p.close()
    println(b.toByteArray.map(x => x & 255).mkString(","))
    val c = new ByteArrayOutputStream
    val q = new PrintStream(c, false, "UTF-16")
    q.print("A"); q.print("B"); q.close()
    println(c.toByteArray.map(x => x & 255).mkString(","))
