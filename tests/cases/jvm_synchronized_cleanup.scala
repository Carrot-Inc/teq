//> using platform jvm
// Monitor cleanup follows every exit, including nested returns and exceptions.
class Box(val n: Int)

object Main:
  val lock = new Object
  var evaluations = 0
  def receiver(): Object =
    evaluations += 1
    lock
  def early(): Int = lock.synchronized {
    try
      lock.synchronized { return 7 }
    finally println("held in finally: " + Thread.holdsLock(lock))
    0
  }
  def main(args: Array[String]): Unit =
    println(receiver().synchronized { Thread.holdsLock(lock) })
    println(evaluations)
    println(Thread.holdsLock(lock))
    println(10 + lock.synchronized { 2 })
    println(lock.synchronized { 2.5 })
    println(new Box(lock.synchronized { 3 }).n)
    println(early())
    println(Thread.holdsLock(lock))
    try lock.synchronized { throw new RuntimeException("body") }
    catch case e: RuntimeException => println(e.getMessage)
    println(Thread.holdsLock(lock))
    try (null: Object).synchronized { println("unreachable") }
    catch case _: NullPointerException => println("null monitor")
    lock.synchronized { () }
    println(Thread.holdsLock(lock))
