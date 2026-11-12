//> using platform jvm
// `@threadUnsafe` given aliases where a class holds them (lazy_thread_unsafe_given): in an object, in a trait held by
// a class, at a file's top level, an anonymous one, each entered by two threads at once (the initialiser waits three
// seconds at most for the second, and the line says whether both were inside); one whose initialiser reads the
// given itself on one thread. Beside them the two that the annotation does not change: a given with a using
// clause, which is a def, and a local given alias, a local lazy val under its holder's monitor (dotty's
// transformLocalDef whatever the annotations).
import java.util.concurrent.{CountDownLatch, TimeUnit}
import scala.annotation.threadUnsafe

object InObject:
  val entered = new CountDownLatch(2)
  @threadUnsafe given x: Int = { entered.countDown(); entered.await(3, TimeUnit.SECONDS); 42 }

trait Givens:
  val entered = new CountDownLatch(2)
  @threadUnsafe given x: Int = { entered.countDown(); entered.await(3, TimeUnit.SECONDS); 43 }
class InTrait extends Givens

val topEntered = new CountDownLatch(2)
@threadUnsafe given top: Long = { topEntered.countDown(); topEntered.await(3, TimeUnit.SECONDS); 44L }

class Anonymous:
  val entered = new CountDownLatch(2)
  @threadUnsafe given String = { entered.countDown(); entered.await(3, TimeUnit.SECONDS); "45" }
  def read: String = summon[String]

class WithUsing:
  val entered = new CountDownLatch(2)
  given s: String = "s"
  @threadUnsafe given x(using String): Int = { entered.countDown(); entered.await(3, TimeUnit.SECONDS); 46 }
  def read: Int = summon[Int]

class Reentrant:
  var depth = 0
  @threadUnsafe given x: Int = { depth += 1; if depth > 1 then 1 else x + 1 }

def twice[T](entered: CountDownLatch)(read: () => T): String =
  val results = new Array[Any](2)
  val a = new Thread(() => results(0) = read())
  val b = new Thread(() => results(1) = read())
  a.start(); b.start(); a.join(); b.join()
  s"${results(0)} ${results(1)}, both inside: ${entered.getCount == 0}"

@main def run(): Unit =
  println(s"in an object: ${twice(InObject.entered)(() => InObject.x)}")
  val t = new InTrait
  println(s"in a trait held by a class: ${twice(t.entered)(() => t.x)}")
  println(s"at the top level: ${twice(topEntered)(() => top)}")
  val a = new Anonymous
  println(s"anonymous: ${twice(a.entered)(() => a.read)}")
  val u = new WithUsing
  println(s"with a using clause (a def): ${twice(u.entered)(() => u.read)}")
  val r = new Reentrant
  println(s"reading itself: x = ${r.x} after ${r.depth} run(s)")
  var depth = 0
  @threadUnsafe given local: Int = { depth += 1; if depth > 1 then 1 else local + 1 }
  println(s"a local one: $local after $depth run(s)")
