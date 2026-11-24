// A path operand that still throws (a val read through a null reference) beside an operand with an effect, in a value
// class's right-associative extension. scalac lifts an infix call's impure left operand, so its effect comes before
// the right operand's NullPointerException; it reads a path on the left where the call takes it, after the right
// operand. Written out, the operands run as written. A null reference's read is the JVM's NullPointerException (on
// JavaScript it is undefined behaviour), so the program is the JVM's platform.
//> using platform jvm
class Box(val value: Int)

class V(val n: Int) extends AnyVal {
  extension (x: Any) def +:(y: Any): String = "" + y + "|" + x + "|" + n
}

object Main {
  var log = List.empty[String]
  val box: Box = null
  def num(k: Int): Int = { log = log :+ ("n" + k); k }
  def show(label: String, result: String): Unit = println(label + " " + result + " " + log.mkString(","))
  def failed(label: String, e: Throwable): Unit = println(label + " " + e.getClass.getSimpleName + " " + log.mkString(","))
  def infixPathRight(v: V): Unit = { log = Nil; try { import v.*; show("infix, the path right", num(1) +: box.value) } catch { case e: NullPointerException => failed("infix, the path right", e) } }
  def infixPathLeft(v: V): Unit = { log = Nil; try { import v.*; show("infix, the path left", box.value +: num(2)) } catch { case e: NullPointerException => failed("infix, the path left", e) } }
  def writtenPathFirst(v: V): Unit = { log = Nil; try show("written, the path first", v.+:(box.value)(num(1))) catch { case e: NullPointerException => failed("written, the path first", e) } }
  def writtenPathLast(v: V): Unit = { log = Nil; try show("written, the path last", v.+:(num(2))(box.value)) catch { case e: NullPointerException => failed("written, the path last", e) } }
  def main(args: Array[String]): Unit = {
    val v = new V(7)
    infixPathRight(v)
    infixPathLeft(v)
    writtenPathFirst(v)
    writtenPathLast(v)
  }
}
