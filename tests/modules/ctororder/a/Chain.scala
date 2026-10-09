package coa

// Parents' constructor calls in the declared order, their arguments' temporaries evaluated
// where the source has them (named arguments out of order).
def note(s: String): Int =
  print(s + " ")
  s.length

class A(x: Int, y: Int):
  print("A(" + x + "," + y + ") ")

trait T:
  print("T ")

class B(z: Int) extends A(y = note("second") + z, x = note("first")) with T:
  print("B ")

class C extends B(note("third")):
  println("C")
