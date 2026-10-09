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
