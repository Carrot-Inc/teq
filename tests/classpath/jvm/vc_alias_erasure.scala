// jars: scala-library
// An applied alias erases as its expansion does: a parameter alias to its argument, an array of one to the
// argument's array, a value class through it to its underlying type or, as an array's element, its box; an
// unreduced match type behind an alias to its bound over the arguments; a reduced one as it reduces; a curried
// alias with every argument.
class V(val u: Int) extends AnyVal
class W[A](val a: A) extends AnyVal
object Defs {
  type Id[A] = A
  type Arr[A] = Array[A]
  type Nested[A] = Id[Arr[Id[A]]]
  type Uni[A] = A | String
  type R[A] = A match { case Int => String; case _ => Object }
  type AR[A] = R[A]
  type Bound[A] <: Array[A] = A match { case Int => Array[A]; case _ => Array[A] }
  type M[A, B] <: B = A match { case Int => B }
  type N[A] = M[A, String]
  type MA[A] <: A = A match { case _ => A }
  type Cur[A] = [B] =>> B
  def id(x: Id[Int]): Id[String] = "x"
  def arr(x: Arr[Int]): Arr[Int] = x
  def nested(x: Nested[Int]): Nested[Int] = x
  def union(x: Uni[Int]): Uni[String] = "s"
  def reduced(x: R[Int]): R[Int] = x
  def areduced(x: AR[Int]): AR[Int] = x
  def bound[A](x: Bound[A]): Bound[A] = x
  def direct[A](x: M[A, String]): M[A, String] = x
  def nestedBound[A](x: N[A]): N[A] = x
  def primitive[A](x: M[A, Int]): M[A, Int] = x
  def array[A](x: M[A, Array[Int]]): M[A, Array[Int]] = x
  def boxArray(a: Array[Id[V]]): Array[Id[V]] = a
  def genArray[A](a: Array[Id[A]]): Array[Id[A]] = a
  def intArray(a: Array[Id[Int]]): Array[Id[Int]] = a
  def matched[A](a: MA[A]): MA[A] = a
  def concrete(a: MA[Int]): MA[Int] = a
  def genericArr[A](a: Arr[A]): Arr[A] = a
  def value(a: Id[V]): Id[V] = a
  def wrapped(w: W[Id[V]]): W[Id[V]] = w
  def wrappedInt(w: W[Id[Int]]): Id[W[Int]] = w
  def curried(x: Cur[String][Int]): Cur[String][Int] = x
}
object Main {
  def main(args: Array[String]): Unit = {
    val ms = Defs.getClass.getDeclaredMethods.filter(m => m.getName != "writeReplace" && !m.getName.contains("$"))
    ms.map(_.toString.replace("Defs$.", "")).sorted.foreach(println)
    println(Defs.value(new V(3)).u + Defs.boxArray(Array(new V(4)))(0).u + Defs.intArray(Array(5))(0) + Defs.wrapped(new W(new V(6))).a.u)
    println(Defs.id(1) + Defs.nested(Array(7)).mkString + Defs.direct[Int]("d") + Defs.primitive[Int](8) + Defs.concrete(9))
  }
}
