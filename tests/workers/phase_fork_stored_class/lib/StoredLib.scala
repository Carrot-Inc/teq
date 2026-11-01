// A library's inline methods whose bodies make an anonymous class (tests/workers.sh's phase_fork_stored_class).
package storedlib

abstract class Fn:
  def apply(): Int

object Make:
  inline def mk(inline x: Int): Fn = new Fn { def apply() = x + 1 }
  inline def twice(inline x: Int): Fn = new Fn { def apply() = x * 2 }
