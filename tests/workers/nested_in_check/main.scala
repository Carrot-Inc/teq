// A class nested in a local class reads the enclosing class's field, which the enclosing check
// marks as reached through an accessor only at its end; a body demanded inside the check (`later`)
// is published meanwhile, and the nested class with it would have sealed the field before its mark.
// scalac prints 4.
trait T:
  def value: Int
def outer(): Int =
  class C extends T:
    val value: Int = 4
    class Inner:
      def get: Int = C.this.value
    val p: Int = later()
    def inner: Int = new Inner().get
  new C().inner
def later() = 1
@main def run(): Unit = println(outer())
