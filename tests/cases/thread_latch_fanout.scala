//> using platform jvm
// A `CountDownLatch` fan-out: six named threads wait on one start latch, each records its name in a
// `ConcurrentHashMap` and counts a second latch down, which main awaits; a timed await on a latch at
// zero answers at once.
import java.util.concurrent.{ConcurrentHashMap, CountDownLatch, TimeUnit}

object Main:
  def main(args: Array[String]): Unit =
    val n = 6
    val start = new CountDownLatch(1)
    val done = new CountDownLatch(n)
    val names = new ConcurrentHashMap[Int, String]()
    for i <- 0 until n do
      val t = new Thread(() =>
        start.await()
        names.put(i, Thread.currentThread().getName)
        done.countDown(), s"worker-$i")
      t.start()
    println(done.getCount)
    println(start.await(10, TimeUnit.MILLISECONDS))
    start.countDown()
    done.await()
    println(done.getCount)
    for i <- 0 until n do println(s"$i -> ${names.get(i)}")
    println(done.await(10, TimeUnit.MILLISECONDS))
