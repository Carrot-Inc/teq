// A type parameter bounded by a value type cannot be compared with null, as under scalac.
object Main:
  def f[A <: Int](a: A): Boolean = a == null
  def main(args: Array[String]): Unit = println(f(1))
// expect: values of types A and Null cannot be compared with == or !=
