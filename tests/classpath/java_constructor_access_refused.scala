// jars: javactors-lib
// A Java class's protected constructor is no subpackage's, no other package's whose name its package's
// begins, and no creation's in a subclass's body; its package-private one is its package's alone, a
// subclass's parent call included; a parent written without its type arguments is typed ahead of the super
// call, with an argument list or without one (tests/support/java_ctors). teq built each into a program that
// failed when it ran.
// expect: error: constructor Guarded cannot be accessed as a member of Guarded from object InSub
// expect: error: constructor Local cannot be accessed as a member of Local from object InSub
// expect: error: constructor Guarded cannot be accessed as a member of Guarded from object Prefix
// expect: error: constructor Guarded cannot be accessed as a member of Guarded from class D
// expect: error: constructor Local cannot be accessed as a member of Local from class E
// expect: error: constructor Local cannot be accessed as a member of Local from object Out
// expect: error: constructor Boxed cannot be accessed as a member of Boxed from object Out
// expect: error: constructor Shelf cannot be accessed as a member of Shelf from class F
// expect: 9 errors found
package jlib.sub:
  object InSub:
    def a = new jlib.Guarded(1)
    def b = new jlib.Local(2)

package jlibx:
  object Prefix:
    def a = new jlib.Guarded(1)

package other:
  class D extends jlib.Guarded(1):
    def mk = new jlib.Guarded(7)
  class E extends jlib.Local(1)
  object Out:
    def a = new jlib.Local(3)
    def b = new jlib.Local(4) { def y = 1 }
    def c = new jlib.Boxed("s") {}
  class F extends jlib.Shelf
