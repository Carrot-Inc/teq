class Ops:
  class Meters(val n: Int) extends AnyVal
def uses(o: Ops): Unit =
  println(Array.empty[o.Meters].length)
// Left as it is told today: the tag of a value class that is a member of a class, which scalac refuses to
// declare (E075). A class named through a path has its class's tag; this one stays refused until its
// declaration is.
// expect: 4:11: error: No ClassTag available for o.Meters
// expect: 1 error found
