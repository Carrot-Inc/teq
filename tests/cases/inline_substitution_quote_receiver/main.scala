// A quote of a macro's splice that names a member of the inline method's class (`'x` of the
// field `x`) reads the receiver's where the body expands: a stable receiver's, one the expansion
// binds to a proxy (`new Num(5)`), an object's. scalac prints the lines of the .expected file.
@main def run(): Unit =
  val n = new Num(3)
  println(n.power(0))
  println(n.power(5))
  println(new Num(5).power(2))
  println(Two.power(3))
