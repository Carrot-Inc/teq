package text
val e = { println("  eager ext"); 1 }
extension (x: Int) def plus1: Int = x + 1
