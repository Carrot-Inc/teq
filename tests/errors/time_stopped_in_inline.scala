// A build that stops with errors names, under --time, the phase it stopped in after the phases it timed: here the
// inlining phase, which runs once the typing has reported no error and reports the expansion's.
// teq: --time
// expect: error: not positive
// expect: 1 error found
// expect: stopped in inline: 1 error
import scala.compiletime.error

object Main:
  inline def positive(inline n: Int): Int = if n > 0 then n else error("not positive")
  def main(args: Array[String]): Unit =
    println(positive(0))
