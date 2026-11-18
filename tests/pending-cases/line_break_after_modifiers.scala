// A definition's modifiers stand on the line before its keyword. scalac prints 1.
object O:
  private[O]
  val x = 1
  def get = x
@main def main(): Unit = println(O.get)
