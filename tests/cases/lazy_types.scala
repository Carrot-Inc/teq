// Lazy vals of every erased kind, as members and as locals, each computed once (dotty's LazyVals over the erased
// type: a member's value boxed into its Object container and NullValue for null, a local's LazyInt ... LazyShort or
// LazyRef holder), and the adaptations around them: a Unit whose initialiser has an effect, Nothing, null, a value
// class over a primitive and over a reference, a value class initialising a lazy Any, a null unboxed, an unbox
// that erasure removes, an alias, a union and an intersection.
class VI(val x: Int) extends AnyVal
class VS(val s: String) extends AnyVal

object Count:
  var n = 0
  def tick[T](v: T): T = { n += 1; v }

class Kinds:
  type A = Array[Int]
  lazy val z: Boolean = Count.tick(true)
  lazy val b: Byte = Count.tick(1.toByte)
  lazy val s: Short = Count.tick(2.toShort)
  lazy val c: Char = Count.tick('c')
  lazy val i: Int = Count.tick(4)
  lazy val l: Long = Count.tick(5L << 40)
  lazy val f: Float = Count.tick(6.5f)
  lazy val d: Double = Count.tick(7.25)
  lazy val u: Unit = Count.tick(println("unit effect"))
  lazy val nl: Null = Count.tick(null)
  lazy val str: String = Count.tick(null)
  lazy val vi: VI = Count.tick(new VI(8))
  lazy val vs: VS = Count.tick(new VS("nine"))
  lazy val anyVc: Any = Count.tick(new VI(10))
  lazy val arr: A = Count.tick(Array(11, 12))
  lazy val union: Int | String = Count.tick(13)
  lazy val inter: Runnable & Serializable = Count.tick(null)
  lazy val unboxed: Int = Count.tick[Any](null).asInstanceOf[Int]
  lazy val boxedAny: Any = Count.tick[Any](14).asInstanceOf[Int]
  lazy val boxedNull: Any = Count.tick[Any](null).asInstanceOf[Int]
  lazy val nothing: Nothing = { Count.tick(0); throw new IllegalStateException("nothing") }

object Main:
  def twice(label: String, read: => Any): Unit =
    val first = read
    val second = read
    println(s"$label $first ${first == second}")

  def main(args: Array[String]): Unit =
    val k = new Kinds
    twice("z", k.z); twice("b", k.b); twice("s", k.s); twice("c", k.c)
    twice("i", k.i); twice("l", k.l); twice("f", k.f); twice("d", k.d)
    twice("u", k.u); twice("nl", k.nl); twice("str", k.str)
    twice("vi", k.vi.x); twice("vs", k.vs.s); twice("anyVc", k.anyVc.asInstanceOf[VI].x)
    twice("arr", k.arr.toList); twice("union", k.union); twice("inter", k.inter)
    twice("unboxed", k.unboxed); twice("boxedAny", k.boxedAny); twice("boxedNull", k.boxedNull)
    println(try k.nothing catch case e: IllegalStateException => "nothing threw " + e.getMessage)
    println(try k.nothing catch case e: IllegalStateException => "nothing threw again " + e.getMessage)
    println(s"initialisers run: ${Count.n}")

    Count.n = 0
    lazy val lz: Boolean = Count.tick(false)
    lazy val lb: Byte = Count.tick(-1.toByte)
    lazy val ls: Short = Count.tick(-2.toShort)
    lazy val lc: Char = Count.tick('x')
    lazy val li: Int = Count.tick(-4)
    lazy val ll: Long = Count.tick(-5L << 40)
    lazy val lf: Float = Count.tick(-6.5f)
    lazy val ld: Double = Count.tick(-7.25)
    lazy val lu: Unit = Count.tick(println("local unit effect"))
    lazy val lnull: String = Count.tick(null)
    lazy val lvi: VI = Count.tick(new VI(-8))
    lazy val lvs: VS = Count.tick(new VS("local nine"))
    twice("lz", lz); twice("lb", lb); twice("ls", ls); twice("lc", lc)
    twice("li", li); twice("ll", ll); twice("lf", lf); twice("ld", ld)
    twice("lu", lu); twice("lnull", lnull); twice("lvi", lvi.x); twice("lvs", lvs.s)
    println(s"local wide arithmetic ${li + ld} ${ll + li}")
    println(s"local initialisers run: ${Count.n}")
