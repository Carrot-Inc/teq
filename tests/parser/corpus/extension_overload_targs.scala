// Two overloaded extensions on an object, called directly on the object with explicit type
// arguments: the alternative whose type parameters take them is the one meant.
trait Inst[K, T]:
  def name: String
final class Impl[K, T](val name: String) extends Inst[K, T]
object Ops:
  extension [A, B](p: (A, B)) def unify: A | B = p._1
  extension [I[k, t] <: Inst[k, t], K, T](inst: I[K, Option[T]]) def unify: I[K, T] = inst.asInstanceOf[I[K, T]]
object Main:
  def main(args: Array[String]): Unit =
    val i = new Impl[String, Option[Int]]("i")
    println(Ops.unify[Impl, String, Int](i).name)
    println(Ops.unify[Int, String]((1, "a")))
