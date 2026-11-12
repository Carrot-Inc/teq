// A Scala wildcard argument of a Java class compared with one
// bounded by `Object`: dotty reads the Java parameter's `Object`
// bound as `FromJavaObject`, so `isSubArgs` meets the argument
// `? <: Any` with `paramBounds(E)`, `? <: FromJavaObject`
// (TypeComparer.scala 1898; `TypeBounds.&`, Types.scala 5763
// and 5767 both give that side), and `FromJavaObject <: Object`
// holds: scalac 3.8.4 accepts and prints 1 and 2.
def f(l: java.util.List[?]): java.util.List[? <: Object] = l
def g(m: java.util.Map[String, ?]): java.util.Collection[? <: Object] = m.values()
@main def javaWildcardObjectBound(): Unit =
  val l = new java.util.ArrayList[String]()
  l.add("a")
  val m = new java.util.HashMap[String, Int]()
  m.put("a", 1)
  m.put("b", 2)
  println(f(l).size)
  println(g(m).size)
