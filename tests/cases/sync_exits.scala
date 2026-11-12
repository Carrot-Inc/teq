// Every way out of a `synchronized` block keeps the program's result: a return from inside, from nested blocks and
// from beside operands on the stack, through a `finally` inside and outside, an exception caught outside, a
// non-local return from a lambda inside (dotty's genSynchronized with its cleanups, genReturn and genLoadTry).
// tests/cases/sync_exits_jvm checks the monitor after each on the JVM, and `boundary`/`break` across a block, which
// teq's JavaScript std does not have.
// teq: --wconf msg=Non.local.returns.are.no.longer.supported:s

object Main:
  val lock = new Object
  val other = new Object
  var log = List.empty[String]
  def note(s: String): Unit = log = s :: log

  def early(c: Boolean): Int = lock.synchronized { if c then return 5; 2 }
  def underOperands(c: Boolean): Int = 1 + lock.synchronized { if c then return 5; 2 }
  def wide(c: Boolean): Long = 1L + lock.synchronized { if c then return 7L; 2L }
  def nested(c: Boolean): Int = lock.synchronized { other.synchronized { if c then return 10; 20 } }
  def finallyInside(c: Boolean): Int = lock.synchronized { try { if c then return 1; 2 } finally note("inner finally") }
  def finallyOutside(c: Boolean): Int = try lock.synchronized { if c then return 3; 4 } finally note("outer finally")
  def throwsInside(): Int = lock.synchronized(throw new RuntimeException("inside"))
  def nonLocal(xs: List[Int]): Int =
    xs.foreach(x => lock.synchronized { if x > 1 then return x })
    -1

  def main(args: Array[String]): Unit =
    println(s"${early(true)} ${early(false)}")
    println(s"${underOperands(true)} ${underOperands(false)}")
    println(s"${wide(true)} ${wide(false)}")
    println(s"${nested(true)} ${nested(false)}")
    println(s"${finallyInside(true)} ${finallyInside(false)}")
    println(s"${finallyOutside(true)} ${finallyOutside(false)}")
    println(try throwsInside() catch case e: RuntimeException => e.getMessage)
    println(nonLocal(List(1, 2, 3)))
    println(log.reverse.mkString(", "))
