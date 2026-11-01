// `Constructors.intoConstr` reads a by-name constructor parameter's thunk for its accessor in the constructor's code
// (after `ElimByName`): the constructor calls its parameter, and a closure there captures that parameter, not the
// instance (tests/support/byname_ctor/list.sh).
package bn

class A(s: => Int):
  val saved = s
  val twice = s + s
  val closure = () => s

@main def run(): Unit =
  val a = A(3)
  println(s"${a.saved} ${a.twice} ${a.closure()}")
