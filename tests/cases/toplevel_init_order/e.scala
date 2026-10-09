package oe
val e = { println("eager e"); 1 }
extension (x: Int) def plus(y: Int): Int = x + y
