// expect: 9:33: error: value noSuch is not a member of Int
// expect: 1 error found
// A local inline method called before its definition, whose definition fails the check: the
// call checks the definition first and is the plain call of the failed record, the body's error
// reported once, at the definition, as scalac 3.8.4 reports it (E008), and never at the call,
// which the retype path's expansion of the failed body added.
def host(): Int =
  val first = bad(1)
  inline def bad(x: Int): Int = x.noSuch
  first
@main def main(): Unit = println(host())
