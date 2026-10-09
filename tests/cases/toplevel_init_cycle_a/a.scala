package cy
// Two files whose initialisers read each other's vals: the second file's initialiser runs inside
// the first's, which the recursive read does not enter again. What the recursive read gives is
// the documented difference, so the vals are read and not printed.
val a: Int = { println("init a"); val seen = b; 1 }
def fa: Int = 0
