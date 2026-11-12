//> using platform jvm
// Adapted from scala3 tests/run/i16806.scala (Apache-2.0, see tests/scala3/README.md); replaced: the joins of 300 ms
// left unchecked and the workers' exceptions left to the default handler, so that a worker's exception or a worker
// unjoined within its deadline fails main.
//scalajs: --skip
import java.util.concurrent.Semaphore

object Repro {

  case object DFBit
  final class DFError extends Exception("")
  final class DFType[+T](val value: T | DFError) extends AnyVal

  def asIR(dfType: DFType[DFBit.type]): DFBit.type = dfType.value match
  case dfTypeIR: DFBit.type => dfTypeIR
  case err: DFError         => throw new DFError

  object Holder {
    val s = new Semaphore(1, false)
    final lazy val Bit = {
      s.release()
      new DFType[DFBit.type](DFBit)
    }
  }

  val failures = java.util.concurrent.ConcurrentLinkedQueue[String]()

  @main
  def Test =
    val a = new Thread() {
      override def run(): Unit =
        try
          Holder.s.acquire()
          val x = Holder.Bit.value
          assert(x.isInstanceOf[DFBit.type])
          println("Success")
        catch case e: Throwable => failures.add(s"a: $e"); ()
    }
    val b = new Thread() {
      override def run(): Unit =
        try
          Holder.s.acquire()
          val x = Holder.Bit.value
          assert(x.isInstanceOf[DFBit.type])
          println("Success")
        catch case e: Throwable => failures.add(s"b: $e"); ()
    }
    a.start()
    b.start()
    a.join(10000)
    b.join(10000)
    if a.isAlive || b.isAlive then failures.add("a worker did not finish")
    if !failures.isEmpty then throw new AssertionError("workers failed: " + failures)
}