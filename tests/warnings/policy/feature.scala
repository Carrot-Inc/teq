implicit def convert(x: Int): String = x.toString
@main def run(): Unit = println(convert(1))
