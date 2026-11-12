//> using platform jvm
// `@threadUnsafe` given aliases where a class holds them (lazy_thread_unsafe_given): in an object, in a trait held by
// a class, at a file's top level, an anonymous one, each entered by two threads at once; one whose initialiser reads
// the given itself on one thread. Beside them the two that the annotation does not change: a given with a using
// clause, which is a def, and a local given alias, a local lazy val under its holder's monitor (dotty's
// transformLocalDef whatever the annotations).
import java.util.concurrent.CountDownLatch
import scala.annotation.threadUnsafe

object InObject:
  val entered = new CountDownLatch(2)
  @threadUnsafe given x: Int = { entered.countDown(); entered.await(); 42 }

trait Givens:
  val entered = new CountDownLatch(2)
  @threadUnsafe given x: Int = { entered.countDown(); entered.await(); 43 }
class InTrait extends Givens

val topEntered = new CountDownLatch(2)
@threadUnsafe given top: Long = { topEntered.countDown(); topEntered.await(); 44L }

class Anonymous:
  val entered = new CountDownLatch(2)
  @threadUnsafe given String = { entered.countDown(); entered.await(); "45" }
  def read: String = summon[String]

class WithUsing:
  val entered = new CountDownLatch(2)
  given s: String = "s"
  @threadUnsafe given x(using String): Int = { entered.countDown(); entered.await(); 46 }
  def read: Int = summon[Int]

class Reentrant:
  var depth = 0
  @threadUnsafe given x: Int = { depth += 1; if depth > 1 then 1 else x + 1 }

def twice[T](read: () => T): String =
  val results = new Array[Any](2)
  val a = new Thread(() => results(0) = read())
  val b = new Thread(() => results(1) = read())
  a.start(); b.start(); a.join(); b.join()
  s"${results(0)} ${results(1)}"

@main def run(): Unit =
  println(s"in an object: ${twice(() => InObject.x)}")
  val t = new InTrait
  println(s"in a trait held by a class: ${twice(() => t.x)}")
  println(s"at the top level: ${twice(() => top)}")
  val a = new Anonymous
  println(s"anonymous: ${twice(() => a.read)}")
  val u = new WithUsing
  println(s"with a using clause (a def): ${twice(() => u.read)}")
  val r = new Reentrant
  println(s"reading itself: x = ${r.x} after ${r.depth} run(s)")
  var depth = 0
  @threadUnsafe given local: Int = { depth += 1; if depth > 1 then 1 else local + 1 }
  println(s"a local one: $local after $depth run(s)")
