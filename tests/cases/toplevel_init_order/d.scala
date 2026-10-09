package od
val e = { println("eager d"); 1 }
def withDefault(x: Int = { println("default"); 1 }): Int = x
