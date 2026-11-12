//> using platform jvm
// A thread made from a function literal notifies the instance that waits for it: the literal's unqualified
// `synchronized` and `notifyAll` are the enclosing instance's, so the waiter is woken and the program ends.
class R {
  def waitNotify(): String = {
    var ready = false
    val t = new Thread(() => synchronized { ready = true; notifyAll() })
    synchronized { t.start(); while (!ready) wait() }
    t.join(); "notified"
  }
}
@main def run(): Unit = println(new R().waitNotify())
