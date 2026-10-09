def p(i: Int*) = i.sum
val h2 = i => p(i)
val f = x => x + 1
@main def run(): Unit = println(h2(1))
