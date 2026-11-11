// A polymorphic function type's `ClassTag` is its function class's wherever the refinement's parent has a stable
// erasure (`hasStableErasure`, read apart from the erasure itself): an intersection with an array of a type parameter
// is stable, as an applied class is.
import scala.reflect.ClassTag
object Main:
  def f[A]: ClassTag[(PolyFunction & Array[A]) { def apply(x: Int): Int }] = summon[ClassTag[(PolyFunction & Array[A]) { def apply(x: Int): Int }]]
  def main(args: Array[String]): Unit =
    println(f[Int])
    println(f[String].runtimeClass.getName)
