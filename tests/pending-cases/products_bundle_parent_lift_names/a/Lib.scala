package bna

def note(s: String): Int =
  print(s.concat(" "))
  s.length

class A(x: Int, y: Int, z: Int):
  print("A(".concat(x.toString).concat(",").concat(y.toString).concat(",").concat(z.toString).concat(") "))

class B(w: Int) extends A(note("one"), z = note("three") + w, y = note("two"))

class D extends A(y = 7, z = note("d1"), x = note("d2"))
