package lctb

@main def run(): Unit =
  println(lct.Lib.deep(17))
  println(lct.Lib.counter(3))
  println(lct.Lib.calls(4))
  println(new lct.Host(10).run(1))
