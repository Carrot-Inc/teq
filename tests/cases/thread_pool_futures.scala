//> using platform jvm
// A fixed pool of three threads: submitted callables' futures, `invokeAll` in order, the pool's
// threads named as the JDK's default factory names them, a task's exception wrapped in
// `ExecutionException`, `shutdown` and `awaitTermination`; a `CompletableFuture` completed by
// another thread and chained.
import java.util.concurrent.{Callable, CompletableFuture, ExecutionException, Executors, TimeUnit}
import scala.jdk.CollectionConverters.*

object Main:
  def main(args: Array[String]): Unit =
    val pool = Executors.newFixedThreadPool(3)
    val futures = (1 to 5).map(i => pool.submit[Int](() => i * i))
    println(futures.map(_.get).mkString(","))
    val calls = (1 to 3).map(i => (() => s"call $i"): Callable[String]).asJava
    println(pool.invokeAll(calls).asScala.map(_.get).mkString(","))
    println(pool.submit[String](() => Thread.currentThread().getName).get.matches("pool-1-thread-[123]"))
    val failed = pool.submit[Int](() => throw new IllegalArgumentException("bad"))
    try failed.get
    catch case e: ExecutionException => println("ExecutionException: " + e.getCause)
    pool.shutdown()
    println(pool.awaitTermination(1, TimeUnit.SECONDS))
    val cf = new CompletableFuture[String]()
    new Thread(() => { Thread.sleep(20); cf.complete("done"); () }).start()
    println(cf.thenApply(_.toUpperCase).get)
    println(CompletableFuture.supplyAsync(() => 6 * 7).get)
