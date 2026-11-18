// What java.io.PrintStream refuses and what it prints for null: a null String or object prints null, a null char
// array throws; an unknown and an illegal charset name throw UnsupportedEncodingException; a null name, a null
// Charset and a null stream throw NullPointerException at construction.
import java.io.*
import java.nio.charset.Charset
object Probe:
  def attempt(label: String)(f: => Unit): Unit =
    try { f; println(label + "=ok") }
    catch case x: Exception => println(label + "=" + x.getClass.getSimpleName)
  def main(args: Array[String]): Unit =
    val b = new ByteArrayOutputStream
    val p = new PrintStream(b)
    attempt("string") { p.print(null: String) }
    attempt("object") { p.print(null: Any) }
    attempt("chars") { p.print(null: Array[Char]) }
    println("bytes=" + b.toString("UTF-8"))
    attempt("unknown") { new PrintStream(b, false, "NO-SUCH-CHARSET"); () }
    attempt("illegal") { new PrintStream(b, false, "bad name!"); () }
    attempt("null-name") { new PrintStream(b, false, null: String); () }
    attempt("null-charset") { new PrintStream(b, false, null: Charset); () }
    attempt("null-output") { new PrintStream(null: OutputStream); () }
