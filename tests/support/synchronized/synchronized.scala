// The fixture of tests/support/synchronized/list.sh: `synchronized` blocks of a value, of Unit and of a class literal.
package sync

class Blocks:
  def value(lock: AnyRef): Int = lock.synchronized(41 + 1)
  def unit(lock: AnyRef): Unit = lock.synchronized(println("inside"))
  def literal(): String = classOf[Blocks].synchronized("held")
  def own(): Long = synchronized(42L)
