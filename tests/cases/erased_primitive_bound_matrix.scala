// A type parameter bounded by a primitive erases to it (`TypeErasure`), so an erased call's `null` it
// returns is unboxed at the method's result; a union of two primitives erases to `Object`, unboxed at the
// consumer (the review's `UnionMatrix`).
object Main:
 type Bound = Int | Long
 def a[A <: Int | Long](m: java.util.HashMap[String,A]): A = m.get("x")
 def b[A <: Int | String](m: java.util.HashMap[String,A]): A = m.get("x")
 def c[A <: AnyVal](m: java.util.HashMap[String,A]): A = m.get("x")
 def d[A <: Bound](m: java.util.HashMap[String,A]): A = m.get("x")
 def e[A <: Int](m: java.util.HashMap[String,A]): A = m.get("x")
 def main(args: Array[String]): Unit =
  val m = new java.util.HashMap[String,Int]()
  val aa: Int = a(m); val bb: Int = b(m); val cc: Int = c(m); val dd: Int = d(m); val ee: Int = e(m)
  println(List(aa,bb,cc,dd,ee))
  println(List(a(m)+1,b(m)+1,c(m)+1,d(m)+1,e(m)+1))
