import scala.compiletime.constValue

object Sizes:
  def pair: Int = constValue[Tuple.Size[(Int, Int)]]
