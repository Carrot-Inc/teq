// A `ClassTag` is synthesized wherever the type's erasure is stable (`hasStableErasure`: an array of a type parameter
// is, through its class), and is the class of the type's erasure (`TypeErasure.erasure`), an array of a type
// parameter's being `Object`: so an intersection or union with such an array, refined or not, tags `Object`.
import scala.reflect.ClassTag
trait S
object Main:
  def g1[A] = summon[ClassTag[Array[A] & S]]
  def g2[A] = summon[ClassTag[(Array[A] & S) { def foo: Int }]]
  def g3[A] = summon[ClassTag[Array[A] | S]]
  def main(args: Array[String]): Unit =
    println(g1[Int].runtimeClass.getName)
    println(g2[Int].runtimeClass.getName)
    println(g3[Int].runtimeClass.getName)
