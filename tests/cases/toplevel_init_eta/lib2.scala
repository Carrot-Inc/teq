package et2
val e = { println("eager 2"); 1 }
// A default whose JavaScript is a function of its own, written before the def's body checks
// the initialiser.
def withTry(x: Int = try 1 finally println("default")): Int = x
