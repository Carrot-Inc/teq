package overload_using

// Overloads of one name, one of a using clause, one of an ordinary one, each called with its own clause: a class's
// `apply` through a value, and an object's method. (Nested objects, which have no static forwarders: scalac's two
// Scala.js builds order a top-level object's forwarders otherwise, which tests/modules-scalac-split.sh lists.)
object Cases:
  object Ops:
    def f(using s: String)(x: Int): String = "ops-using:" + s + x
    def f(s: String): Int => String = x => "ops-plain:" + s + x

  object Lib:
    class V:
      def apply(using s: String)(x: Int): String = "using:" + s + x
      def apply(s: String): Int => String = x => "plain:" + s + x
    val v = new V
    def literal: String = v(using "V")(3)
    def plain: String = v("P")(4)
    def givenArg: String =
      given s: String = "G"
      v(using s)(5)
    def opsLiteral: String = Ops.f(using "O")(6)
    def opsPlain: String = Ops.f("Q")(7)
