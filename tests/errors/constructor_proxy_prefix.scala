// expect: 8:26: error: Outer is not a valid class prefix, since it is not an immutable path
// expect: 1 error found

class Outer:
  class A(i: Int)
  object A
object Use:
  def h() = new Outer().A(1)
