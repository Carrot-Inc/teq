// jars: scala-library
// std: lean scala-library
// A Scala class extending a Java class whose constructors are alternatives (`AbstractList`,
// slf4j's `LegacyAbstractLogger` with its protected constructor), `Object` as a type, and a write
// to a Java field (`Point.x`), which is a `putfield`, not a setter call.
class Q3 extends java.util.AbstractList[String]:
  def get(i: Int) = "v" + i
  def size() = 2

class E extends Object:
  override def toString = "E"

object Main:
  def main(args: Array[String]): Unit =
    val q = new Q3
    println(q.get(1) + q.size())
    val o: Object = new E
    println(o)
    val p = new java.awt.Point(1, 2)
    p.x = 5
    println(p.x)
