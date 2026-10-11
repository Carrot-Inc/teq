package app

// The upstream trait's implicit object, inherited by a downstream object, is the given of its
// own singleton type through the object: imported, written in full, and inside the object.
object TestAssertions extends lib.Assertions:
  def inside: String = check + " / " + icheck(true)

@main def run(): Unit =
  locally {
    import TestAssertions.*
    println(check)
    println(icheck(1 == 1))
  }
  println(TestAssertions.check + " / " + TestAssertions.icheck(2 == 1))
  println(TestAssertions.inside)
  println(summon[TestAssertions.UseDefault.type] eq TestAssertions.UseDefault)
