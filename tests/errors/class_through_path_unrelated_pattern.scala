class Ops:
  object nums:
    class Box(val v: Int)
    case class Pt(x: Int)
    trait Mark
  class Top(val v: Int)
  case class Wide(x: Int)
  def inside(b: nums.Box): Int = b match { case _: nums.Pt => 1; case _ => 0 }
def uses(o: Ops, p: Ops): Unit =
  val b: o.nums.Box = new o.nums.Box(1)
  println(b match { case _: o.nums.Pt => 1; case _ => 0 })
  println(b match { case o.nums.Pt(x) => x; case _ => 0 })
  println(b match { case _: p.nums.Pt => 1; case _ => 0 })
  println(b match { case _: o.nums.Mark => 1; case _ => 0 })
  println(b match { case _: p.nums.Box => 1; case _ => 0 })
  import o.nums.*
  println(b match { case _: Pt => 1; case _ => 0 })
  val t: Ops#Top = new o.Top(1)
  println(t match { case _: Ops#Wide => 1; case _ => 0 })
// A typed pattern and a case class pattern of a class unrelated to the scrutinee's are refused where the classes are
// named through a path (`TypeTestsCasts.checkSensical`, by the classes): a value's, another instance's, the class's
// own inner object's, an import's, a projection's; a trait the class may have and the same class of another
// instance are no error.
// expect: 8:49: error: unreachable case: type
// expect: 11:26: error: unreachable case: type o.nums.Box and class Pt are unrelated
// expect: 12:26: error: unreachable case: type o.nums.Box and class Pt are unrelated
// expect: 13:26: error: unreachable case: type o.nums.Box and class Pt are unrelated
// expect: 17:26: error: unreachable case: type o.nums.Box and class Pt are unrelated
// expect: 19:26: error: unreachable case: type Ops#Top and class Wide are unrelated
// expect: 6 errors found
// absent: 14:26: error:
// absent: 15:26: error:
