// Small members library bodies reach: Option.orNull, BigInteger.divideAndRemainder, the array's
// length comparisons, `new String` over a char array, java.lang.ref's references, Long.reverse
// and rotateRight.
object Main:
  def main(args: Array[String]): Unit =
    val a: Option[String] = Some("x")
    val b: Option[String] = None
    println(a.orNull)
    println(b.orNull)
    val f: Option[() => Int] = None
    println(f.orNull == null)
    val q = new java.math.BigInteger("100").divideAndRemainder(new java.math.BigInteger("7"))
    println(q(0).toString + " " + q(1).toString)
    val xs = Array(1, 2, 3)
    println(xs.lengthCompare(2) + " " + xs.lengthCompare(3) + " " + xs.lengthCompare(4))
    println(xs.sizeCompare(1) + " " + xs.lengthIs + " " + xs.sizeIs)
    val cs = Array('h', 'e', 'l', 'l', 'o')
    println(new String(cs, 1, 3))
    println(new String(cs))
    println(new String("x") + new String() + "|")
    val ref = new java.lang.ref.WeakReference(cs)
    println(ref.get().length)
    val soft = new java.lang.ref.SoftReference("s")
    println(soft.get())
    soft.clear()
    println(soft.get() == null)
    println(java.lang.Long.reverse(1L) + " " + java.lang.Long.reverse(0x123456789abcdef0L) + " " + java.lang.Long.reverse(-1L))
    println(java.lang.Long.rotateRight(1L, 1) + " " + java.lang.Long.rotateRight(0x123456789abcdef0L, 12))
