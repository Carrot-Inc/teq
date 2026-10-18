// Extension methods named as Object's wait, called with type arguments or inferring them: Object.wait takes no type
// parameters, so a call with type arguments never selects it (dotty's Applications.typedTypeApply), and the
// extension is called, as scalac calls it, on every target.
class Gate
extension (g: Gate)
  def wait[A](value: A): Unit = println("generic " + value)
  def wait[A](values: List[A], label: String): Unit = println(label + " " + values.head)
object Main:
  def main(args: Array[String]): Unit =
    new Gate().wait[Int](42)
    new Gate().wait(List("inferred"), "list")
