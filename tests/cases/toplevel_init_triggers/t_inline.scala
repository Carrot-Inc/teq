package tinl
val e = { println("  eager inline"); 1 }
inline def readE: Int = e + 1
