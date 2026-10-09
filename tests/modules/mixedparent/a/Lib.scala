package pa

// Parents' calls with a positional argument before out-of-order named ones, and with named
// ones in order: the source's evaluation order is the arguments' as written.
def note(s: String): Int =
  print(s + " ")
  s.length

class A(x: Int, y: Int, z: Int):
  print("A(" + x + "," + y + "," + z + ") ")

class B(w: Int) extends A(note("one"), z = note("three") + w, y = note("two")):
  print("B ")

class C extends A(note("p"), note("q"), z = note("r")):
  print("C ")

class D extends A(y = 7, z = note("d1"), x = note("d2")):
  print("D ")
