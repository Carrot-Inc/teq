// expect: 9:11: error: Other is not an enclosing class
// expect: 1 error found
// `C.this` names an enclosing class only, as under scalac.
class Other:
  def name = "other"

class C:
  def name = "c"
  def a = Other.this.name

@main def Main(): Unit = println(C().a)
