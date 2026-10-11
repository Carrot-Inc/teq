// jars: scala-library
// `eq` and `ne` are reference identity (scalac's `ID`, `Object.eq`), a string's and a box's too: a
// substring is no literal (master compared strings by value, `true`), while `==` of two strings and of
// two boxes compares their values. JavaScript's strings and numbers are its primitives, so there `eq` of
// equal strings is true (COMPATIBILITY.md, "Known differences").
class Box(val v: Int):
  override def equals(o: Any) = o match
    case b: Box => b.v == v
    case _ => false
@main def run() =
  val a = "ab".substring(0, 1)
  println(a eq "a")
  println(a eq a)
  println(a ne "a")
  println(a == "a")
  println(a != "a")
  val s: Any = a
  println(s == "a")
  val r: AnyRef = a
  println(r eq "a")
  val i1: Integer = Integer.valueOf(1000)
  val i2: Integer = Integer.valueOf(1000)
  println(i1 eq i2)
  println(i1 == i2)
  val b1 = Box(1)
  println(b1 eq Box(1))
  println(b1 == Box(1))
  println(b1 eq b1)
  val n: String = null
  println(n eq null)
  println(a eq null)
  println(n == null)
