// An enum whose constructor parameter has a default, a case
// leaving it out (`E1`) and one giving it (`E2`). The .expected
// is scalac 3.8.4's output; teq's products build is to build
// the program (`compiler build --target jvm --products`).
// Found by reviewer #13 of the workers exports helper (push 8).
enum E(val w: Int = 11):
  case E1 extends E()
  case E2 extends E(5)

@main def run(): Unit =
  println(E.E1.w)
  println(E.E2.w)
  println(E.values.map(_.w).mkString(","))
