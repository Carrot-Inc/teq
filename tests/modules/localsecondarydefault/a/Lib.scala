package lsd

// A secondary constructor's default of a class nested in a local class (and in a local object), passed by an
// anonymous class over a trait over it: scalac's pickle names it on the class's companion (`LC.this.C.<init>$default$1`),
// a downstream module reading the body takes the enclosing instance the creation passes for it.
object V { def n(k: Int): Int = { println("arg " + k); k } }
object Lib:
  def plain: Int =
    class LC:
      class C(val x: Int, val y: Int) { def this(z: Int = 40) = this(z, V.n(2)); def s: Int = x * 10 + y }
      trait T extends C { val t: Int = 1 }
      def s: Int = (new T {}).s
    new LC().s
  def outer: Int =
    object LO:
      val base: Int = 500
      class C(val x: Int, val y: Int) { def this(z: Int = 40) = this(z, V.n(3)); def s: Int = base * 10 + x * 10 + y }
      trait T extends C { val t: Int = 1 }
      trait U { val u: Int = 3 }
      def s: Int = (new T with U {}).s
    LO.s
