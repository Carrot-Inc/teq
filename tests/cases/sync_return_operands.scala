// A `return` out of a `synchronized` block with operands under it or inside it: of both widths, a constructor's
// (`new` not yet initialised), two blocks among one constructor's arguments, through a `finally` inside the block,
// through nested blocks, from a lambda inside (dotty's genSynchronized and genReturn: the value stored, the jump to
// the block's cleanup, the monitor released; the operands left dead, which teq drops before the jump).
// teq: --wconf msg=Non.local.returns.are.no.longer.supported:s

class Pair(val a: Int, val b: Long):
  def this(a: Int) = this(a, 0L)
  override def toString = s"Pair($a, $b)"
object Main:
  val lock = new Object
  def wide(a: Long, d: Double, s: String): String = s"$a $d $s"
  // operands of both widths under the block
  def depths(n: Int): String =
    wide(10L, 2.5, lock.synchronized { if n == 1 then return "first"; "deep" })
  // operands of both widths inside the block too
  def inside(n: Int): String =
    lock.synchronized { wide(n.toLong, 1.25, if n == 2 then return "second" else "deep") }
  // a constructor's operands under two blocks, a long between them
  def ctor(n: Int): Pair =
    new Pair(lock.synchronized { if n == 1 then return new Pair(-1, -1L); n * 2 }, lock.synchronized { if n == 2 then return new Pair(-2); 9L })
  // a return through a finalizer inside the block, operands inside the try
  def fin(n: Int): String =
    lock.synchronized { try wide(1L, 1.25, if n == 1 then return "fin" else "tried") finally println(s"finally $n") }
  // nested monitors under operands, the inner return leaving both
  def nested(n: Int): Int =
    1 + lock.synchronized { 10 + Main.synchronized { if n > 0 then return n; 100 } }
  // a return from a lambda inside the block (non-local), operands under it
  def viaLambda(xs: List[Int]): Int =
    7 + lock.synchronized { xs.foreach(x => if x > 2 then return x * 100); xs.sum }
  def main(args: Array[String]): Unit =
    println(depths(1)); println(depths(2)); println(inside(2)); println(inside(3))
    println(fin(1)); println(fin(2))
    println(ctor(0)); println(ctor(1)); println(ctor(2))
    println(nested(0)); println(nested(5))
    println(viaLambda(List(1, 2))); println(viaLambda(List(1, 3)))
