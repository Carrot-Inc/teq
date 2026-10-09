// A plain script buffers its output while main runs; leaving through process.exit must not
// lose what was printed.
@main def main(): Unit =
  println("before exit")
  print("no newline")
  js.call(js.global("process"), "exit", 0)
  println("never printed")
