package scala.io:
  trait AnsiColor:
    final val BLACK = "\u001b[30m"
    final val RED = "\u001b[31m"
    final val GREEN = "\u001b[32m"
    final val YELLOW = "\u001b[33m"
    final val BLUE = "\u001b[34m"
    final val MAGENTA = "\u001b[35m"
    final val CYAN = "\u001b[36m"
    final val WHITE = "\u001b[37m"
    final val BLACK_B = "\u001b[40m"
    final val RED_B = "\u001b[41m"
    final val GREEN_B = "\u001b[42m"
    final val YELLOW_B = "\u001b[43m"
    final val BLUE_B = "\u001b[44m"
    final val MAGENTA_B = "\u001b[45m"
    final val CYAN_B = "\u001b[46m"
    final val WHITE_B = "\u001b[47m"
    final val RESET = "\u001b[0m"
    final val BOLD = "\u001b[1m"
    final val UNDERLINED = "\u001b[4m"
    final val BLINK = "\u001b[5m"
    final val REVERSED = "\u001b[7m"
    final val INVISIBLE = "\u001b[8m"

  object AnsiColor extends AnsiColor

  // scala-library's Codec over the charsets of java.nio.charset: a charset by its name, the implicit one of
  // last resort the platform's default.
  class Codec(val charSet: java.nio.charset.Charset):
    def name: String = charSet.name()
    override def toString: String = name

  trait LowPriorityCodecImplicits:
    implicit lazy val fallbackSystemCodec: Codec = Codec.defaultCharsetCodec

  object Codec extends LowPriorityCodecImplicits:
    final val ISO8859: Codec = Codec(java.nio.charset.StandardCharsets.ISO_8859_1)
    final val UTF8: Codec = Codec(java.nio.charset.StandardCharsets.UTF_8)
    def defaultCharsetCodec: Codec = apply(java.nio.charset.Charset.defaultCharset())
    def default: Codec = defaultCharsetCodec
    def apply(encoding: String): Codec = new Codec(java.nio.charset.Charset.forName(encoding))
    def apply(charSet: java.nio.charset.Charset): Codec = new Codec(charSet)
    def fromUTF8(bytes: Array[Byte]): Array[Char] = new String(bytes, java.nio.charset.StandardCharsets.UTF_8).toCharArray
    def toUTF8(cs: CharSequence): Array[Byte] = cs.toString.getBytes(java.nio.charset.StandardCharsets.UTF_8)
    implicit def string2codec(s: String): Codec = apply(s)
    implicit def charset2codec(c: java.nio.charset.Charset): Codec = apply(c)

package scala:
  // The route of the prelude's prints, which Console's are too: calls of a stream (`p` takes the value of a print(x), `l` of a
  // println(x), `e` println() alone); null for the three: the process's output.
  @js("$redirect($0, $1, $2)")
  private[scala] def redirectOut(p: (Any => Unit) | Null, l: (Any => Unit) | Null, e: (() => Unit) | Null): Unit
  // Console's making binds its streams (`java.io.ProcessStream.setOut`): its `out` (0) or `err` (1), `s` as it stands or the
  // stream kept by the first move after an earlier prelude print; the process's own output stream by identity.
  @js("$consoleStream($0, $1)")
  private[scala] def consoleStream(s: java.io.PrintStream, i: Int): java.io.PrintStream
  @js("$isProcessOut($0)")
  private[scala] def isProcessOut(p: java.io.PrintStream): Boolean
  // A print(x)'s text as the prelude prints it, made in the stream's call.
  @js("$printed($0)")
  private[scala] def printedText(x: Any): String

  // `out` is the stream `print` writes to, `System.out` unless `withOut` names another for
  // the length of its body.
  object Console extends io.AnsiColor:
    private var outStream: java.io.PrintStream = null
    private var errStream: java.io.PrintStream = null
    // `out` and `err` are the streams of Console's first use, whatever `System.setOut` and `setErr` do later (the reference
    // binds both once): System.out and System.err as they stand when Console is made, or, where the prelude printed before,
    // as they stood at that print (kept by the first move after it). Under --std scala-library the library's Console binds
    // them as it is made by a named rule (src/typer/check.rs: its `outVar` and `errVar` initialise in its constructor), the
    // first case of a jar object's vals initialising in its constructor, which removes the rule when it lands.
    // (The route is already `firstOut`'s here: a move before the binding moved it, a move after it does not.)
    private val firstOut: java.io.PrintStream = consoleStream(java.lang.System.out, 0)
    private val firstErr: java.io.PrintStream = consoleStream(java.lang.System.err, 1)
    def out: java.io.PrintStream = if outStream == null then firstOut else outStream
    def err: java.io.PrintStream = if errStream == null then firstErr else errStream
    // The route is `out`'s at every moment, so Console's prints are the prelude's: one route for every entry point.
    def println(): Unit = printlnNone()
    def println(x: Any): Unit = printlnImpl(x)
    def print(x: Any): Unit = scala.print(x)
    def flush(): Unit = out.flush()
    // The stream of `withOut`'s body, whichever stream it is (the process's own too: its print goes to the process past the
    // redirect), and after it Console's first `out` (the process's own: no call); the prelude's `print(x)` is its `print(text)` with
    // the text made in the call (scalac's `Console.print`), `println(x)` its `println(x)` with the value (scalac's
    // `println(Object)`), `println()` its `println()`.
    private def reach(s: java.io.PrintStream): Unit =
      outStream = s
      val r = if s == null then firstOut else s
      if s == null && isProcessOut(r) then redirectOut(null, null, null)
      else redirectOut((x: Any) => r.print(printedText(x)), (x: Any) => r.println(x), () => r.println())
    def withOut[T](out: java.io.PrintStream)(thunk: => T): T =
      val saved = outStream
      reach(out)
      try thunk
      finally reach(saved)
    def withErr[T](err: java.io.PrintStream)(thunk: => T): T =
      val saved = errStream
      errStream = err
      try thunk
      finally errStream = saved
