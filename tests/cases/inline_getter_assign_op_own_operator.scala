// A value that has the assignment operator itself keeps it: `h.items += 4` through an inline getter of a
// buffer is the buffer's `+=`, and no setter is called.
package meters

import scala.collection.mutable.ListBuffer

class Holder:
  private val held = ListBuffer[Int]()
  inline def items: ListBuffer[Int] = held
  def items_=(v: ListBuffer[Int]): Unit = throw new IllegalStateException("the setter")

@main def run(): Unit =
  val h = Holder()
  h.items += 4
  h.items ++= List(5, 6)
  println(h.items.sum)
