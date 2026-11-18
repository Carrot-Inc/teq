// An end marker names a definition whose name is written in backquotes. scalac prints 1.
def `a b`(): Int =
  val x = 1
  x
end `a b`
@main def main(): Unit = println(`a b`())
