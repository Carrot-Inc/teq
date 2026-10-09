// The `ClassTag` of a compound type with an opaque type in it: the opaque type erases as
// what it is made of, which the erasure alone sees.
import scala.reflect.ClassTag

object O:
  opaque type X = String
  def x: X = "x"
  opaque type P[A] = List[A]
  def p: P[Int] = List(1)

def tag[T](using t: ClassTag[T]): String = t.runtimeClass.getName

@main def run(): Unit =
  println(tag[O.X | String] + " " + tag[O.X & String] + " " + (summon[ClassTag[O.P[Int] | List[String]]].runtimeClass == classOf[List[Int]]))
  println(Array.fill[O.X | String](2)(O.x).mkString + " " + Array.empty[Array[O.X] | Array[String]].length)
