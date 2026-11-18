// A method's result type on a line further in after its colon, the right side's statements on lines between the
// method's width and the type's: the colon opens no block. scalac prints 1.
object O:
  def f:
      Int =
    val a = 1
    a
@main def main(): Unit = println(O.f)
