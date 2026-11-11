// `A & Pure` and `Pure & A` erase as `A` (TypeErasure.scala:801-804), so their ClassTag is `A`'s class, not `Object`'s.
import scala.reflect.ClassTag
trait Z
object Main:
  type PZ = Z & scala.caps.Pure
  def t[T](using c: ClassTag[T]): String = c.runtimeClass.getName
  def main(args: Array[String]): Unit =
    println(t[Z & scala.caps.Pure])
    println(t[scala.caps.Pure & Z])
    println(t[(Z & scala.caps.Pure) | Null])
    println(t[PZ])
