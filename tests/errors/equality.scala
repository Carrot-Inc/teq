// expect: values of types Int and String cannot be compared with == or !=
// expect: values of types A and String cannot be compared with == or !=
// expect: values of types Some[Int] and String cannot be compared with == or !=
// expect: values of types List[Int] and String cannot be compared with == or !=
// expect: values of types Color and String cannot be compared with == or !=
// expect: values of types P and String cannot be compared with == or !=
// Default mode of scalac 3.8.4: a comparison is rejected when one side has a reflexive CanEqual
// (primitives, String, Seq, Set) and the types are unrelated. The last four lines are legal.
class A
class B
enum Color:
  case Red, Green
enum Shape:
  case Circle
case class P(x: Int)

@main def run(): Unit =
  println(1 == "s")
  println((new A) == "s")
  println(Some(1) == "s")
  println(List(1) == "s")
  println(Color.Red == "Red")
  println(P(1) == "x")
  println((new A) == (new B))
  println(P(1) == (new A))
  println(Color.Red == Shape.Circle)
  println(Some(1) == None)
