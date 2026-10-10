// expect: 12:42: error: type mismatch: found (x : I), required Sub.this.I
// expect: 23:11: error: value same is not a member of b.Term: the extension method takes c.Term
// expect: 2 errors found
// An inner class through a subclass's `this` is not the one through the enclosing instance's
// (`TypeComparer` compares two class `this` types by their class), and an extension's receiver
// through a leading using parameter is checked at the type the given's path gives it
// (`Applications.extMethodApply`).
class O(val n: Int):
  class I:
    def value = n
  class Sub extends O(22):
    def wrong(x: O.this.I): Sub.this.I = x

class Ctx:
  class Term
extension (using c: Ctx)(x: c.Term)
  def same: Boolean = true

@main def run(): Unit =
  val a = new Ctx
  val b = new Ctx
  given c: a.type = a
  println(new b.Term().same)
