// An abstract def, val or var where nothing can implement it, a member of an object or an anonymous class or a
// local, is scalac's E067 (dotty's `Checking.checkWellFormed`); master accepted every one.
// expect: 10:7: error: Declaration of method f not allowed here: only classes can have declared but undefined members
// expect: 11:7: error: Declaration of value v not allowed here: only classes can have declared but undefined members
// expect: 12:7: error: Declaration of variable x not allowed here: only classes can have declared but undefined members
// expect: 15:21: error: Declaration of method g not allowed here: only classes can have declared but undefined members
// expect: 16:21: error: Declaration of variable y not allowed here: only classes can have declared but undefined members
// expect: 17:15: error: Declaration of method h not allowed here: only classes can have declared but undefined members
object O {
  def f: Int
  val v: Int
  var x: Int
}
trait T
val t = new T { def g: Int }
val u = new T { var y: Int }
def m = { def h: Int; 1 }
