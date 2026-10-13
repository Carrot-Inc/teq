//> using platform jvm
// split-watch: jvm
// A given object of a class instance is a module of the instance, made once whatever the threads
// (dotty's `LazyVals.transformMemberDefThreadSafe`: `Evaluating` while one thread makes it, a
// `Waiting` latch the others wait on): a second thread reading it while the first makes it waits
// for that one.
trait T { def n: Int }
class C:
  @volatile var made = 0
  given u: T with
    made += 1
    Thread.sleep(400)
    def n = 42
@main def run(): Unit =
  val c = new C
  val a = Thread(() => println(c.u.n))
  a.start()
  Thread.sleep(100)
  val b = Thread(() => println(c.u.n))
  b.start()
  a.join()
  b.join()
  println(c.made)
