package two
val more: List[Int] = { println("init two"); List(2) }
inline def cons(inline x: Int): List[Int] = x :: more
