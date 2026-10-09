def p(i: Int*) = i.sum
val h2 = i => p(i)
@main def run(): Unit = println(h2(1))
