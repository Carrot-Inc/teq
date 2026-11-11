// A build that stops with errors names, under --time, the phase it stopped in after the phases it timed: here the
// typing, whose error keeps the inlining phase from running.
// teq: --time
// expect: error: Found:    (1 : Int)
// expect: Required: String
// expect: 1 error found
// expect: stopped in type: 1 error
import scala.compiletime.error

object Main:
  inline def positive(inline n: Int): Int = if n > 0 then n else error("not positive")
  def main(args: Array[String]): Unit =
    println(1: String)
    println(positive(0))
