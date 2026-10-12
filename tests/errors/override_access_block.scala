// expect: 19:24: error: method f has weaker access privileges than method f in trait C; it should be at least protected
// expect: 32:37: error: method g has weaker access privileges than method g in class Outer; it should be at least protected
// An override's access is checked as dotty's `RefChecks.isOverrideAccessOK` checks it: public, or
// protected where the overridden member is and bound inside the overridden member's boundary;
// a class's `protected[C]` member may be overridden in `C`'s companion with `protected[C]` there
// (`Base`), but not for a class of a block, whose companion dotty does not take there (`C`), nor
// across unrelated classes.
trait Base:
  protected[Base] def f: Int
object Base:
  class Child extends Base:
    protected[Base] def f: Int = 42

def m: Int =
  trait C:
    protected[C] def f: Int
  object C:
    class C2 extends C:
      protected[C] def f: Int = 42
      def test = f
  C.C2().test

class Outer:
  protected[Outer] def g: Int = 1
  protected def h: Int = 2
  class Nested extends Outer:
    protected[Outer] override def g: Int = 3
    override protected def h: Int = 4
    def all = g + h

class Elsewhere extends Outer:
  protected[Elsewhere] override def g: Int = 5
