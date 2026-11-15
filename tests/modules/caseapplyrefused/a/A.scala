package car

// Case classes with a secondary constructor, read by module b from this module's products: applied there with
// the secondary's arguments they are refused, as in one build (tests/errors/case_apply_primary_alone.scala).
case class V(x: Int):
  def this(a: Int, b: Int) = this(a + b)
case class S private (x: Int):
  def this(a: Int, b: Int) = this(a + b)
trait Outer:
  case class In(x: Int):
    def this() = this(7)
