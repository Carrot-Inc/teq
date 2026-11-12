//> using platform jvm
// Nested `synchronized` blocks, one monitor entered again inside another's: each is held where its block is open and
// given back at its end, the blocks have their value, and no update is lost under them.
import java.util.concurrent.{CountDownLatch, Executors}

object Nest:
  val a = new Object
  val b = new Object
  private var n = 0
  def held: String = s"a ${Thread.holdsLock(a)}, b ${Thread.holdsLock(b)}"
  def bump(): Unit = a.synchronized {
    b.synchronized {
      a.synchronized {
        val seen = n
        Thread.onSpinWait()
        n = seen + 1
      }
    }
  }
  def value: Int = n

@main def run(): Unit =
  import Nest.*
  println(s"outside: $held")
  val r = a.synchronized {
    println(s"in a: $held")
    val inner = b.synchronized {
      println(s"in a, in b: $held")
      a.synchronized { println(s"in a, in b, in a again: $held"); 1 } + 1
    }
    println(s"in a, after b: $held")
    inner + 1
  }
  println(s"outside again: $held; the blocks' value: $r")
  val threads = 8
  val each = 20000
  val ex = Executors.newFixedThreadPool(threads)
  val start = new CountDownLatch(1)
  val done = new CountDownLatch(threads)
  for _ <- 1 to threads do
    ex.submit(new Runnable {
      def run(): Unit =
        try { start.await(); var i = 0; while i < each do { Nest.bump(); i += 1 } }
        finally done.countDown()
    })
  start.countDown(); done.await(); ex.shutdown()
  println(s"${Nest.value} of ${threads * each} increments kept under the nested blocks")
