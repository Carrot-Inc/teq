package cnru

import scala.deriving.Mirror
import scala.compiletime.constValue

// A mirror an inline method takes for its types alone: the whole build keeps the binding of the
// companion's reference, which runs an initialiser in its source's judgement, and the build over
// the products drops it, as scalac 3.8.4's two builds do (`getstatic Models$G$.MODULE$`).
object Use:
  inline def arity[A](using m: Mirror.ProductOf[A]): Int = constValue[Tuple.Size[m.MirroredElemTypes]]
  def g: Int = arity[cnr.Models.G]
  def n: Int = arity[cnr.Models.N]

@main def run(): Unit = println(s"${Use.g} ${Use.n}")
