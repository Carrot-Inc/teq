// A type parameter bounded by a union of two primitive classes (`A <: Int | Long`) erases to `Object`, the erased
// lub of the union's sides (dotty's `TypeErasure`, `TypeComparer.orType` with `isErased`), so a call returning it
// may give `null`: consumed as an `Int` it is unboxed to the zero (`Erasure.Boxing.adaptToType`), printed as `Any`
// it stays `null`.
object Main:
 def read[A <: Int | Long](m: java.util.HashMap[String, A]): A = m.get("x")
 def main(args: Array[String]): Unit =
  val m = new java.util.HashMap[String, Int]()
  println(read(m))
  val x: Int = read(m)
  println(x)
  println(x + 1)
