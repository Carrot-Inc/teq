package vcse

object Main:
  def main(args: Array[String]): Unit =
    val v = new V(1)
    println(v.withThis)
    println(v.fromLambda)
    println(v.fromLocalDef)
