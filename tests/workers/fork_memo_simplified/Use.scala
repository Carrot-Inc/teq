object Use { def g(): String = { val y = Lib.mk(); y.get.name + Lib.m(1) } }
@main def main(): Unit = println(Use.g())
