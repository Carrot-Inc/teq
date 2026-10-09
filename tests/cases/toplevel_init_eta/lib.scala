package et
val e = { println("eager"); 1 }
def f(x: Int): Int = x
def g(x: Int, y: Int): Int = x + y
