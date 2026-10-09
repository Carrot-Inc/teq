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

package scala:
  // `out` is the stream `print` writes to, `System.out` unless `withOut` names another for
  // the length of its body.
  object Console extends io.AnsiColor:
    private var outStream: java.io.PrintStream = null
    private var errStream: java.io.PrintStream = null
    def out: java.io.PrintStream = if outStream == null then java.lang.System.out else outStream
    def err: java.io.PrintStream = if errStream == null then java.lang.System.err else errStream
    def println(): Unit = if outStream == null then printlnImpl("") else outStream.println()
    def println(x: Any): Unit = if outStream == null then printlnImpl(x) else outStream.println(x)
    def print(x: Any): Unit = if outStream == null then scala.print(x) else outStream.print(x)
    def flush(): Unit = out.flush()
    def withOut[T](out: java.io.PrintStream)(thunk: => T): T =
      val saved = outStream
      outStream = if out eq java.lang.System.out then null else out
      try thunk
      finally outStream = saved
    def withErr[T](err: java.io.PrintStream)(thunk: => T): T =
      val saved = errStream
      errStream = err
      try thunk
      finally errStream = saved
