class Ops:
  class Meters(val n: Int) extends AnyVal
def uses(o: Ops): Unit =
  println(Array.empty[o.Meters].length)
// A value class that is a member of a class, which scalac refuses to declare (E075) and teq too. scalac reports
// that alone; teq refuses the tag besides (a class named through a path has its class's tag).
// expect: 2:9: error: Value classes may not be a member of another class
// expect: 4:11: error: No ClassTag available for o.Meters
// expect: 2 errors found
