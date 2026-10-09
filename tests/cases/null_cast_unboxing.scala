// `x.asInstanceOf[Int]` of a reference unboxes: a null gives the type's zero, as scalac's
// `BoxesRunTime.unboxToInt` and its kin do, on every target; the receiver runs once.
inline def castTo[T](x: Any): T = x.asInstanceOf[T]
inline def opt[T](v: Any): Option[T] = if v.isInstanceOf[T] then Some(v.asInstanceOf[T]) else None

object O:
  opaque type I = Int
  def f(a: Any): Int = a.asInstanceOf[I] + 1

var evaluated = 0
def any(tag: String): Any =
  evaluated += 1
  println("evaluated " + tag)
  null

@main def run(): Unit =
  val a: Any = null
  println(a.asInstanceOf[Int])
  println(a.asInstanceOf[Int] + 1)
  println(a.asInstanceOf[Boolean])
  println(a.asInstanceOf[Long] + 1L)
  println(java.lang.Double.doubleToLongBits(a.asInstanceOf[Double]))
  println(java.lang.Float.floatToIntBits(a.asInstanceOf[Float]))
  println(a.asInstanceOf[Char].toInt)
  println(a.asInstanceOf[Short] + 1)
  println(a.asInstanceOf[Byte] + 1)
  println(any("int").asInstanceOf[Int] + 1)
  println(any("long").asInstanceOf[Long] * 2L)
  println(!any("bool").asInstanceOf[Boolean])
  println(any("double").asInstanceOf[Double] == 0.0)
  println(evaluated)
  val r: AnyRef = null
  println(r.asInstanceOf[Int])
  val s: String = null
  println(s.asInstanceOf[Any].asInstanceOf[Int])
  val n: java.lang.Integer = null
  println(n.asInstanceOf[Int] - 1)
  val b: Any = 7
  println(b.asInstanceOf[Int] + 1)
  val xs: List[Any] = List(null, 2)
  println(xs.map(x => x.asInstanceOf[Int] + 1))
  println(castTo[Int](null) + 1)
  println(castTo[Long](any("inline long")) + 1L)
  println(castTo[Boolean](null))
  println(castTo[String](null))
  println(castTo[Char](null).toInt)
  println(opt[Int](3))
  println(opt[Int]("x"))
  println(opt[String]("x"))
  println(opt[Boolean](true))
  println((a.asInstanceOf[Int]: Any))
  val short: Any = (5: Short)
  println((short.asInstanceOf[Short]: Any) == (5: Short))
  println(O.f(null))
  println(a.asInstanceOf[0] + 1)
  println(a.asInstanceOf[Int & AnyVal] + 1)
