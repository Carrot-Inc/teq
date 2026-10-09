// A lazy val whose value is null is computed once, as scalac's holders keep the computed state
// apart from the value: a class's, an object's, an inherited trait's, a top-level one, a local
// one (read directly, through a lambda and through a local class), with a reference, an array
// or a type parameter's result, a lazy given; and an initialiser that throws leaves it to be
// computed again at the next read.
var count = 0
def fresh[A](): A = { count += 1; null.asInstanceOf[A] }

lazy val top: String = fresh()

class C:
  lazy val value: String = fresh()
  lazy val array: Array[String] = fresh()

class Gen[A]:
  lazy val value: A = fresh()

trait T:
  lazy val value: String = fresh()
class D extends T

object O:
  lazy val value: String = fresh()
  given lazyGiven: String = fresh()

object Retry:
  var attempts = 0
  lazy val value: String =
    attempts += 1
    if attempts == 1 then throw new RuntimeException("first")
    null

def twice(label: String, read: () => Any): Unit =
  val before = count
  val a = read()
  val b = read()
  println(s"$label: $a $b ${count - before}")

@main def run(): Unit =
  val c = new C
  twice("class", () => c.value)
  twice("array", () => c.array)
  val g = new Gen[List[Int]]
  twice("generic", () => g.value)
  val d = new D
  twice("trait", () => d.value)
  twice("object", () => O.value)
  twice("given", () => summon[String](using O.lazyGiven))
  twice("top", () => top)
  lazy val local: String = fresh()
  twice("local", () => local)
  lazy val ints: Array[Int] = fresh()
  twice("local array", () => ints)
  lazy val captured: String = fresh()
  class Reader:
    def read: String = captured
  val r = new Reader
  twice("local class", () => r.read)
  try println(Retry.value)
  catch case e: RuntimeException => println("threw " + e.getMessage)
  twice("retry", () => Retry.value)
  println(Retry.attempts)
  var localAttempts = 0
  lazy val localRetry: String =
    localAttempts += 1
    if localAttempts == 1 then throw new RuntimeException("local first")
    null
  try println(localRetry)
  catch case e: RuntimeException => println("threw " + e.getMessage)
  twice("local retry", () => localRetry)
  println(localAttempts)
  println(count)
