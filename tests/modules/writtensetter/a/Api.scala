package wsa

// A local class's var beside a written overload of its setter's name: a call of the overload is
// that call, not the var's assignment, and the converted class leaves out the var's generated
// setter alone.
object Api:
  def f(): String =
    object State:
      var n = 0
      def n_=(s: Short): Unit = n = 99
    State.n_=(1.toShort)
    val a = State.n
    State.n = 5
    s"$a ${State.n}"

  def g(): Int =
    class Box:
      var v = 1
      def v_=(s: String): Unit = v = s.length
    val b = new Box
    b.v_=("four")
    b.v
