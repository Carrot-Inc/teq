package ain

// Inline methods that read, call and assign protected members their class inherits: a
// downstream expands them through the accessors the class gets for them (`inline$ain$Lib$$secret`,
// `inline$secret` in a final class), which scalac's downstream calls on teq's class files.
class Parent:
  protected def secret(x: Int): Int = x + 1
  protected var count: Int = 0

class Lib extends Parent:
  inline def f(x: Int): Int = secret(x)
  inline def bump(): Int = { count += 1; count }

final class Fin extends Parent:
  inline def g(x: Int): Int = secret(x) * 2

// A private member an inline method reads, the method expanded downstream over a subclass's
// instance: the member is the class's own, which the accessor's body reads on the class.
class Holder:
  private val x = 7
  inline def n: Int = x

class Gen[A](a: A):
  private val held: A = a
  private var reads = 0
  inline def get: A = { reads += 1; held }
  def seen = reads
