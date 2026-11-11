// An operator assignment to a parenthesized indexed element, variable, field or getter with a setter is the same assignment
// without the parentheses: the left operand's own tree (Desugar.binop, `Parens(arg) => arg`).
import scala.collection.mutable.ArrayBuffer
class O { var f = 1 }
class Box:
  private var n = 0
  def value: Int = n
  def value_=(x: Int): Unit = { n = x }
  def bump(): Unit = { (value) += 10 }
object T:
  def main(args: Array[String]): Unit =
    val a = Array(1, 2)
    (a(0)) += 1
    var x = 1
    (x) += 1
    val o = O()
    (o.f) += 2
    val b = ArrayBuffer(List(1))
    (b(0)) ++= List(2)
    println(a.toList)
    println(x)
    println(o.f)
    println(b)
    val box = Box()
    (box.value) += 1
    println(box.value)
    box.bump()
    println(box.value)
