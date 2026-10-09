// A local object is a term of its block, like a val: one name, one definition.
object A:
  def f: Int =
    val a: Int = 1
    object a:
      def g = 1
    a
// expect: a is already defined as value a
