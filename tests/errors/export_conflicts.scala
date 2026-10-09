// expect: 15:10: error: x is exported and defined in object B
// expect: 19:10: error: x is exported twice, for two different definitions
// expect: 25:10: error: cannot export greet: it would override the concrete member greet of trait Greeter
// expect: 3 errors found
object A:
  def x = "A.x"
  def only = "only A"
object R:
  def x = "R.x"
object Impl:
  def greet = "hello from Impl"
trait Greeter:
  def greet: String = "trait greet"
object B:
  export A.x
  def x = "B.x"
object C:
  export A.*
  export R.*
object Fine:
  def x = "own"
  export A.*
  export A.only
object G extends Greeter:
  export Impl.greet
