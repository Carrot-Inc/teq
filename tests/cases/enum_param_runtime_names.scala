// An enum's parameter that implements a trait's member, so that it is a field behind an accessor,
// named `name`, `ordinal` or like a JavaScript function's own (`length`), which a
// case's parameter overrides: each value's argument and each case's own, as scalac prints them, plain and when the enum
// is a function.
trait Named:
  def name: String
trait Sized:
  def length: Int
trait Placed:
  def ordinal: Int

enum E(val name: String) extends Named:
  case A extends E("alpha")
  case B(override val name: String) extends E(name + "!")
  case C(k: Int) extends E("c" * k)

enum F(val name: String) extends Named with (Int => String):
  def apply(i: Int) = name * i
  case A extends F("alpha")
  case B(override val name: String) extends F(name + "!")

enum L(val length: Int) extends Sized:
  case A extends L(3)
  case B(override val length: Int) extends L(length + 1)

@main def run(): Unit =
  println(E.A.name)
  println((E.A: Named).name)
  println(E.A)
  println(E.B("b").name)
  println(E.C(2).name)
  println(F.A.name)
  println(F.A(2))
  println(F.B("b").name)
  println(F.B("b")(2))
  println(L.A.length)
  println(L.B(5).length)
