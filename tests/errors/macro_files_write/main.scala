// expect: main.scala:4:33: error: not supported yet in a macro: a macro writes no file: its expansions run in parallel and may be run again, so its effects on disk would have no order
// expect: 1 error found

@main def run(): Unit = println(Macros.written)
