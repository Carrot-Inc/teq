// expect: 15:9: error: non-private method foo in class Inner refers to private value x
// expect: 23:11: error: non-private method foo in class Inner refers to private value x
// expect: 2 errors found

// dotty's tests/neg/leaks.scala (3.8.4): `Checking.checkNoPrivateLeaks` (TypeAssigner#avoidPrivateLeaks at a
// member's completion) rejects a non-private member whose signature names a private value invisible where the member
// is visible (`isLeaked`: its owner's scope, or its linked class's, does not enclose the member's access boundary),
// as scalac's message names them: `non-private ${sym.showLocated} refers to private ${tp.symbol}`. A local class's
// member's boundary is the method that owns the class, inside `Enclosing`, so `y` does not leak; a private alias is
// expanded away (`k`), and a private member is not checked (`p`).
class Outer {
  private val x: Int = 1

  class Inner {
    def foo: x.type = x // error: non-private method foo refers to private value x in its type signature
  }
}

class Outer3Neg {
  def meth: Unit = {
    class Inner {
      private val x: Int = 1
      def foo: x.type = x // error
    }
  }
}

class Enclosing {
  private val y: Int = 3
  private type T = Int
  def k: T = 1
  private def p: y.type = y
  def m: Int = {
    class L {
      def f: y.type = y
    }
    1
  }
}
