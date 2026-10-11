// expect: 16:26: error: method f has weaker access privileges than method f in trait C; it should be at least protected
// expect: 23:17: error: method h has weaker access privileges than method h in trait P; it should be at least protected
// expect: 2 errors found

// An override relaxes the access boundary of what it overrides, dotty's
// `RefChecks.isOverrideAccessOK`: a qualified boundary is relaxed by one containing it, or by the
// companion object of the class that bounds it where that class is not local to a block.
package accn

object X:
  def m: Int =
    trait C:
      protected[C] def f: Int
    object C:
      class C2 extends C:
        protected[C] def f: Int = 42
        def test = f
    C.C2().test

trait P:
  protected[accn] def h: Int
class Q extends P:
  protected def h: Int = 2
