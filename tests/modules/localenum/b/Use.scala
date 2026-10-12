package leb

@main def run(): Unit =
  println(lea.Lib.value(1) eq lea.Lib.value(1))
  println(lea.Lib.read(7) + lea.Lib.read(8))
  println(lea.Lib.ordinal())
