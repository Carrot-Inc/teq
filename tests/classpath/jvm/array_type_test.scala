// jars: scala-library
// On the JVM a type test against an array type looks at the element: `isInstanceOf` and a typed pattern make
// the test scalac's erasure writes (the array's erased class; under a union or an intersection each member's own),
// and neither is told uncheckable where the element is a class.

object R01_kinds:
  class Box(val n: Int)
  class Base
  class Sub extends Base
  case class K(n: Int)
  final class F
  object O
  trait T
  enum E { case A, B }
  def run(): Unit =
    class Loc
    val s: Any = Array("a")
    val b: Any = Array(new Box(1))
    val sub: Any = Array(new Sub)
    val base: Any = Array[Base](new Sub)
    val ints: Any = Array(1, 2)
    val longs: Any = Array(1L)
    val anys: Any = Array[Any](1, "a")
    val refs: Any = Array[AnyRef]("a")
    val integers: Any = Array[Integer](1)
    val nul: Any = null
    val notArr: Any = "a"
    val loc: Any = Array(new Loc)
    println("r01 unrelated " + s.isInstanceOf[Array[Box]])
    println("r01 same " + b.isInstanceOf[Array[Box]])
    println("r01 sub-as-base " + sub.isInstanceOf[Array[Base]])
    println("r01 base-as-sub " + base.isInstanceOf[Array[Sub]])
    println("r01 int-as-long " + ints.isInstanceOf[Array[Long]])
    println("r01 int-as-int " + ints.isInstanceOf[Array[Int]])
    println("r01 long-as-int " + longs.isInstanceOf[Array[Int]])
    println("r01 int-as-any " + ints.isInstanceOf[Array[Any]])
    println("r01 int-as-anyref " + ints.isInstanceOf[Array[AnyRef]])
    println("r01 int-as-anyval " + ints.isInstanceOf[Array[AnyVal]])
    println("r01 int-as-integer " + ints.isInstanceOf[Array[Integer]])
    println("r01 integer-as-int " + integers.isInstanceOf[Array[Int]])
    println("r01 str-as-anyref " + s.isInstanceOf[Array[AnyRef]])
    println("r01 str-as-any " + s.isInstanceOf[Array[Any]])
    println("r01 str-as-string " + s.isInstanceOf[Array[String]])
    println("r01 any-as-string " + anys.isInstanceOf[Array[String]])
    println("r01 anyref-as-string " + refs.isInstanceOf[Array[String]])
    println("r01 box-as-string " + b.isInstanceOf[Array[String]])
    println("r01 case " + s.isInstanceOf[Array[K]])
    println("r01 final " + s.isInstanceOf[Array[F]])
    println("r01 object " + s.isInstanceOf[Array[O.type]])
    println("r01 trait " + s.isInstanceOf[Array[T]])
    println("r01 enum " + s.isInstanceOf[Array[E]])
    println("r01 null " + nul.isInstanceOf[Array[Box]])
    println("r01 not-array " + notArr.isInstanceOf[Array[Box]])
    println("r01 negated " + !s.isInstanceOf[Array[Box]])
    println("r01 in-if " + (if s.isInstanceOf[Array[Box]] then "box" else "other"))
    println("r01 in-and " + (s.isInstanceOf[Array[Box]] && s != null))

object R02_static:
  class Box(val n: Int)
  def run(): Unit =
    val xs: Array[String] = Array("a")
    val bs: Array[Box] = Array(new Box(1))
    val is: Array[Int] = Array(1)
    val ref: AnyRef = Array("a")
    val obj: Object = Array(1)
    val seqLike: Array[? <: AnyRef] = Array("a")
    println("r02 static-str-as-box " + xs.isInstanceOf[Array[Box]])
    println("r02 static-str-as-str " + xs.isInstanceOf[Array[String]])
    println("r02 static-str-as-anyref " + xs.isInstanceOf[Array[AnyRef]])
    println("r02 static-box-as-box " + bs.isInstanceOf[Array[Box]])
    println("r02 static-int-as-long " + is.isInstanceOf[Array[Long]])
    println("r02 static-int-as-int " + is.isInstanceOf[Array[Int]])
    println("r02 anyref-as-box " + ref.isInstanceOf[Array[Box]])
    println("r02 anyref-as-str " + ref.isInstanceOf[Array[String]])
    println("r02 object-as-int " + obj.isInstanceOf[Array[Int]])
    println("r02 object-as-box " + obj.isInstanceOf[Array[Box]])
    println("r02 wild-as-box " + seqLike.isInstanceOf[Array[Box]])
    println("r02 wild-as-str " + seqLike.isInstanceOf[Array[String]])

object R02B_serializable:
  class Box(val n: Int)
  def run(): Unit =
    val ser: java.io.Serializable = Array("a")
    val cl: Cloneable = Array(new Box(1))
    println("r02 serializable-as-box " + ser.isInstanceOf[Array[Box]])
    println("r02 serializable-as-str " + ser.isInstanceOf[Array[String]])
    println("r02 cloneable-as-box " + cl.isInstanceOf[Array[Box]])
    println("r02 cloneable-as-str " + cl.isInstanceOf[Array[String]])

object R04_nested:
  class Box(val n: Int)
  def run(): Unit =
    val ii: Any = Array(Array(1))
    val ll: Any = Array(Array(1L))
    val ss: Any = Array(Array("a"))
    val bb: Any = Array(Array(new Box(1)))
    val sss: Any = Array(Array(Array("a")))
    val s: Any = Array("a")
    println("r04 int2-as-long2 " + ii.isInstanceOf[Array[Array[Long]]])
    println("r04 int2-as-int2 " + ii.isInstanceOf[Array[Array[Int]]])
    println("r04 long2-as-long2 " + ll.isInstanceOf[Array[Array[Long]]])
    println("r04 str2-as-box2 " + ss.isInstanceOf[Array[Array[Box]]])
    println("r04 box2-as-box2 " + bb.isInstanceOf[Array[Array[Box]]])
    println("r04 str2-as-anyref2 " + ss.isInstanceOf[Array[Array[AnyRef]]])
    println("r04 str2-as-anyref1 " + ss.isInstanceOf[Array[AnyRef]])
    println("r04 int2-as-anyref1 " + ii.isInstanceOf[Array[AnyRef]])
    println("r04 int2-as-any2 " + ii.isInstanceOf[Array[Array[Any]]])
    println("r04 str1-as-str2 " + s.isInstanceOf[Array[Array[String]]])
    println("r04 str3-as-str2 " + sss.isInstanceOf[Array[Array[String]]])
    println("r04 str3-as-str3 " + sss.isInstanceOf[Array[Array[Array[String]]]])
    println("r04 str3-as-box3 " + sss.isInstanceOf[Array[Array[Array[Box]]]])

object R05_special:
  class Box(val n: Int)
  class V(val x: Int) extends AnyVal
  object Op:
    opaque type Arr = Array[Box]
    def mk(b: Box): Arr = Array(b)
    def inside(x: Any): Boolean = x.isInstanceOf[Arr]
  def run(): Unit =
    val s: Any = Array("a")
    val us: Any = Array((), ())
    val vs: Any = Array(new V(1))
    val is: Any = Array(1)
    val ia: Any = IArray(new Box(1))
    val op: Any = Op.mk(new Box(1))
    println("r05 unit-unrelated " + s.isInstanceOf[Array[Unit]])
    println("r05 unit-same " + us.isInstanceOf[Array[Unit]])
    println("r05 value-unrelated " + s.isInstanceOf[Array[V]])
    println("r05 value-same " + vs.isInstanceOf[Array[V]])
    println("r05 value-int " + is.isInstanceOf[Array[V]])
    println("r05 opaque-inside-unrelated " + Op.inside(s))
    println("r05 opaque-inside-same " + Op.inside(op))
    println("r05 nothing " + s.isInstanceOf[Array[Nothing]])
    println("r05 null-element " + s.isInstanceOf[Array[Null]])
    println("r05 char " + (Array(97.toChar): Any).isInstanceOf[Array[Char]])
    println("r05 char-as-int " + (Array(97.toChar): Any).isInstanceOf[Array[Int]])
    println("r05 boolean " + (Array(true): Any).isInstanceOf[Array[Boolean]])
    println("r05 double-as-float " + (Array(1.5): Any).isInstanceOf[Array[Float]])
    println("r05 byte-as-short " + (Array(1.toByte): Any).isInstanceOf[Array[Short]])
    println("r05 wild-int " + (Array(1): Any).isInstanceOf[Array[?]])
    println("r05 wild-not-array " + ("a": Any).isInstanceOf[Array[?]])

object R06_union_inter:
  class Box(val n: Int)
  class Other
  def run(): Unit =
    val s: Any = Array("a")
    val b: Any = Array(new Box(1))
    val str: Any = "a"
    println("r06 union-with-class-unrelated " + s.isInstanceOf[Array[Box] | String])
    println("r06 union-with-class-str " + str.isInstanceOf[Array[Box] | String])
    println("r06 union-with-class-box " + b.isInstanceOf[Array[Box] | String])
    println("r06 union-of-arrays-str " + s.isInstanceOf[Array[Box] | Array[String]])
    println("r06 union-of-arrays-int " + (Array(1): Any).isInstanceOf[Array[Box] | Array[String]])
    println("r06 inter-serializable-unrelated " + s.isInstanceOf[Array[Box] & java.io.Serializable])
    println("r06 inter-serializable-box " + b.isInstanceOf[Array[Box] & java.io.Serializable])
    println("r06 inter-cloneable-box " + b.isInstanceOf[Array[Box] & Cloneable])
    println("r06 element-union " + s.isInstanceOf[Array[Box | String]])
    println("r06 element-inter " + s.isInstanceOf[Array[Box & java.io.Serializable]])
    println("r06 union-of-arrays-other " + (Array(new Other): Any).isInstanceOf[Array[Box] | Array[String]])
    println("r06 union-array-int-other " + (Array(new Other): Any).isInstanceOf[Array[Box] | Array[Int]])
    println("r06 union-array-int-int " + (Array(1): Any).isInstanceOf[Array[Box] | Array[Int]])
    println("r06 union-three " + s.isInstanceOf[Array[Box] | Array[Int] | String])
    def pat(x: Any): String = x match
      case _: (Array[Box] | String) => "box-or-string"
      case _ => "other"
    def patInter(x: Any): String = x match
      case _: (Array[Box] & java.io.Serializable) => "box-and-serializable"
      case _ => "other"
    println("r06 pattern-union " + pat(s) + " " + pat(b) + " " + pat(str) + " " + pat(1))
    println("r06 pattern-inter " + patInter(s) + " " + patInter(b))

object R07_alias_inline:
  class Box(val n: Int)
  type AB = Array[Box]
  type Arr[T] = Array[T]
  inline def test(x: Any): Boolean = x.isInstanceOf[Array[Box]]
  inline def testT[T](x: Any): Boolean = x.isInstanceOf[Array[T]]
  transparent inline def testTr(x: Any): Boolean = x.isInstanceOf[Array[Box]]
  inline def testAlias(x: Any): Boolean = x.isInstanceOf[AB]
  def run(): Unit =
    val s: Any = Array("a")
    val b: Any = Array(new Box(1))
    println("r07 alias-unrelated " + s.isInstanceOf[AB])
    println("r07 alias-same " + b.isInstanceOf[AB])
    println("r07 alias-applied-unrelated " + s.isInstanceOf[Arr[Box]])
    println("r07 alias-applied-same " + b.isInstanceOf[Arr[Box]])
    println("r07 inline-unrelated " + test(s))
    println("r07 inline-same " + test(b))
    println("r07 inline-param-unrelated " + testT[Box](s))
    println("r07 inline-param-same " + testT[Box](b))
    println("r07 inline-param-int " + testT[Int](s))
    println("r07 transparent-unrelated " + testTr(s))
    println("r07 inline-alias-unrelated " + testAlias(s))
    println("r07 lambda-unrelated " + List(s).map(_.isInstanceOf[Array[Box]]))
    println("r07 lambda-same " + List(b).map(_.isInstanceOf[Array[Box]]))
    val f: Any => Boolean = _.isInstanceOf[Array[Box]]
    println("r07 function-value " + f(s) + " " + f(b))
    lazy val lz = s.isInstanceOf[Array[Box]]
    println("r07 lazy " + lz)

object R12_class_member:
  class Box(val n: Int)
  trait Checker:
    def boxes(x: Any): Boolean = x.isInstanceOf[Array[Box]]
  class Impl extends Checker:
    val field: Boolean = (Array("a"): Any).isInstanceOf[Array[Box]]
    def nested(x: Any): Boolean =
      def inner(y: Any) = y.isInstanceOf[Array[Box]]
      inner(x)
    def byName(x: => Any): Boolean = x.isInstanceOf[Array[Box]]
  object Stat:
    val s: Any = Array("a")
    val test: Boolean = s.isInstanceOf[Array[Box]]
  case class Wrap(x: Any):
    def boxes: Boolean = x.isInstanceOf[Array[Box]]
  extension (x: Any) def isBoxes: Boolean = x.isInstanceOf[Array[Box]]
  def run(): Unit =
    val s: Any = Array("a")
    val b: Any = Array(new Box(1))
    val c = new Impl
    println("r12 trait-method " + c.boxes(s) + " " + c.boxes(b))
    println("r12 field " + c.field)
    println("r12 nested-def " + c.nested(s) + " " + c.nested(b))
    println("r12 by-name " + c.byName(s) + " " + c.byName(b))
    println("r12 object-val " + Stat.test)
    println("r12 case-class " + Wrap(s).boxes + " " + Wrap(b).boxes)
    println("r12 extension " + s.isBoxes + " " + b.isBoxes)
    println("r12 while " + { var n = 0; var k = 0; while k < 2 do { if (if k == 0 then s else b).isInstanceOf[Array[Box]] then n += 1; k += 1 }; n })
    println("r12 guard " + (s match { case x if x.isInstanceOf[Array[Box]] => "boxes"; case _ => "other" }))

object R13_checkable:
  class Box(val n: Int)
  class G[T](val x: T)
  def run(): Unit =
    val s: Any = Array("a")
    val l: Any = Array(List(1))
    val g: Any = Array(new G(1))
    val ls: Array[List[String]] = Array(List("a"))
    val ao: Array[Object] = Array(List("a"))
    println("r13 list-wild-from-any " + l.isInstanceOf[Array[List[?]]])
    println("r13 g-wild-from-any " + g.isInstanceOf[Array[G[?]]])
    println("r13 list-string-from-static " + ls.isInstanceOf[Array[List[String]]])
    println("r13 list-wild-from-static " + ls.isInstanceOf[Array[List[?]]])

object R17_case_layers:
  trait Holder:
    type M = Array[Int] | Array[Array[String]]
    type N[T] = Array[Array[T]]
  object H extends Holder
  type TopU = Array[Int] | Array[Array[String]]
  def nested[T](x: Any): Boolean = x.isInstanceOf[Array[Array[T]] @unchecked]
  def bound[T <: Array[Int]](x: Any): Boolean = x.isInstanceOf[Array[T] @unchecked]
  def memberNested[T](x: Any): Boolean = x.isInstanceOf[H.N[T] @unchecked]
  def unionGeneric[T](x: Any): Boolean = x.isInstanceOf[(Array[Array[T]] | Array[Int]) @unchecked]
  def memberUnion(x: Any): Boolean = x.isInstanceOf[H.M]
  def topUnion(x: Any): Boolean = x.isInstanceOf[TopU]
  def wild2(x: Any): Boolean = x.isInstanceOf[Array[Array[?]]]
  def any2(x: Any): Boolean = x.isInstanceOf[Array[Array[Any]]]
  def three(x: Any): Boolean = x.isInstanceOf[Array[Array[Array[Int]]]]
  def inter(x: Any): Boolean = x.isInstanceOf[Array[Int] & java.io.Serializable]
  def run(): Unit =
    val inputs: List[(String, Any)] = List("s1" -> Array("a"), "s2" -> Array(Array("a")), "i1" -> Array(1), "i2" -> Array(Array(1)), "o1" -> Array[Any](Array(1), "x"), "i3" -> Array(Array(Array(1))), "str" -> "x")
    def row(name: String)(f: Any => Boolean): Unit = println(name + " " + inputs.map((n, v) => n + "=" + f(v)).mkString(" "))
    row("layers-nested")(nested[Int])
    row("layers-bound")(bound[Array[Int]])
    row("layers-member-nested")(memberNested[Int])
    row("layers-union-generic")(unionGeneric[Int])
    row("layers-member-union")(memberUnion)
    row("layers-top-union")(topUnion)
    row("layers-wild2")(wild2)
    row("layers-any2")(any2)
    row("layers-three")(three)
    row("layers-inter")(inter)

object P01_kinds:
  class Box(val n: Int)
  class Base
  class Sub extends Base
  case class K(n: Int)
  final class F
  object O
  trait T
  enum E { case A, B }
  def run(): Unit =
    class Loc
    val s: Any = Array("a")
    val b: Any = Array(new Box(1))
    val sub: Any = Array(new Sub)
    val base: Any = Array[Base](new Sub)
    val ints: Any = Array(1, 2)
    val longs: Any = Array(1L)
    val anys: Any = Array[Any](1, "a")
    val refs: Any = Array[AnyRef]("a")
    val integers: Any = Array[Integer](1)
    val nul: Any = null
    val notArr: Any = "a"
    val loc: Any = Array(new Loc)
    println("p01 unrelated " + (s match { case _: (Array[Box]) => true; case _ => false }))
    println("p01 same " + (b match { case _: (Array[Box]) => true; case _ => false }))
    println("p01 sub-as-base " + (sub match { case _: (Array[Base]) => true; case _ => false }))
    println("p01 base-as-sub " + (base match { case _: (Array[Sub]) => true; case _ => false }))
    println("p01 int-as-long " + (ints match { case _: (Array[Long]) => true; case _ => false }))
    println("p01 int-as-int " + (ints match { case _: (Array[Int]) => true; case _ => false }))
    println("p01 long-as-int " + (longs match { case _: (Array[Int]) => true; case _ => false }))
    println("p01 int-as-any " + (ints match { case _: (Array[Any]) => true; case _ => false }))
    println("p01 int-as-anyref " + (ints match { case _: (Array[AnyRef]) => true; case _ => false }))
    println("p01 int-as-anyval " + (ints match { case _: (Array[AnyVal]) => true; case _ => false }))
    println("p01 int-as-integer " + (ints match { case _: (Array[Integer]) => true; case _ => false }))
    println("p01 integer-as-int " + (integers match { case _: (Array[Int]) => true; case _ => false }))
    println("p01 str-as-anyref " + (s match { case _: (Array[AnyRef]) => true; case _ => false }))
    println("p01 str-as-any " + (s match { case _: (Array[Any]) => true; case _ => false }))
    println("p01 str-as-string " + (s match { case _: (Array[String]) => true; case _ => false }))
    println("p01 any-as-string " + (anys match { case _: (Array[String]) => true; case _ => false }))
    println("p01 anyref-as-string " + (refs match { case _: (Array[String]) => true; case _ => false }))
    println("p01 box-as-string " + (b match { case _: (Array[String]) => true; case _ => false }))
    println("p01 case " + (s match { case _: (Array[K]) => true; case _ => false }))
    println("p01 final " + (s match { case _: (Array[F]) => true; case _ => false }))
    println("p01 object " + (s match { case _: (Array[O.type]) => true; case _ => false }))
    println("p01 trait " + (s match { case _: (Array[T]) => true; case _ => false }))
    println("p01 enum " + (s match { case _: (Array[E]) => true; case _ => false }))
    println("p01 null " + (nul match { case _: (Array[Box]) => true; case _ => false }))
    println("p01 not-array " + (notArr match { case _: (Array[Box]) => true; case _ => false }))
    println("p01 negated " + !(s match { case _: (Array[Box]) => true; case _ => false }))
    println("p01 in-if " + (if (s match { case _: (Array[Box]) => true; case _ => false }) then "box" else "other"))
    println("p01 in-and " + ((s match { case _: (Array[Box]) => true; case _ => false }) && s != null))

object P04_nested:
  class Box(val n: Int)
  def run(): Unit =
    val ii: Any = Array(Array(1))
    val ll: Any = Array(Array(1L))
    val ss: Any = Array(Array("a"))
    val bb: Any = Array(Array(new Box(1)))
    val sss: Any = Array(Array(Array("a")))
    val s: Any = Array("a")
    println("p04 int2-as-long2 " + (ii match { case _: (Array[Array[Long]]) => true; case _ => false }))
    println("p04 int2-as-int2 " + (ii match { case _: (Array[Array[Int]]) => true; case _ => false }))
    println("p04 long2-as-long2 " + (ll match { case _: (Array[Array[Long]]) => true; case _ => false }))
    println("p04 str2-as-box2 " + (ss match { case _: (Array[Array[Box]]) => true; case _ => false }))
    println("p04 box2-as-box2 " + (bb match { case _: (Array[Array[Box]]) => true; case _ => false }))
    println("p04 str2-as-anyref2 " + (ss match { case _: (Array[Array[AnyRef]]) => true; case _ => false }))
    println("p04 str2-as-anyref1 " + (ss match { case _: (Array[AnyRef]) => true; case _ => false }))
    println("p04 int2-as-anyref1 " + (ii match { case _: (Array[AnyRef]) => true; case _ => false }))
    println("p04 int2-as-any2 " + (ii match { case _: (Array[Array[Any]]) => true; case _ => false }))
    println("p04 str1-as-str2 " + (s match { case _: (Array[Array[String]]) => true; case _ => false }))
    println("p04 str3-as-str2 " + (sss match { case _: (Array[Array[String]]) => true; case _ => false }))
    println("p04 str3-as-str3 " + (sss match { case _: (Array[Array[Array[String]]]) => true; case _ => false }))
    println("p04 str3-as-box3 " + (sss match { case _: (Array[Array[Array[Box]]]) => true; case _ => false }))

object P06_union_inter:
  class Box(val n: Int)
  class Other
  def run(): Unit =
    val s: Any = Array("a")
    val b: Any = Array(new Box(1))
    val str: Any = "a"
    println("p06 union-with-class-unrelated " + (s match { case _: (Array[Box] | String) => true; case _ => false }))
    println("p06 union-with-class-str " + (str match { case _: (Array[Box] | String) => true; case _ => false }))
    println("p06 union-with-class-box " + (b match { case _: (Array[Box] | String) => true; case _ => false }))
    println("p06 union-of-arrays-str " + (s match { case _: (Array[Box] | Array[String]) => true; case _ => false }))
    println("p06 union-of-arrays-int " + ((Array(1): Any) match { case _: (Array[Box] | Array[String]) => true; case _ => false }))
    println("p06 inter-serializable-unrelated " + (s match { case _: (Array[Box] & java.io.Serializable) => true; case _ => false }))
    println("p06 inter-serializable-box " + (b match { case _: (Array[Box] & java.io.Serializable) => true; case _ => false }))
    println("p06 inter-cloneable-box " + (b match { case _: (Array[Box] & Cloneable) => true; case _ => false }))
    println("p06 element-union " + (s match { case _: (Array[Box | String]) => true; case _ => false }))
    println("p06 element-inter " + (s match { case _: (Array[Box & java.io.Serializable]) => true; case _ => false }))
    println("p06 union-of-arrays-other " + ((Array(new Other): Any) match { case _: (Array[Box] | Array[String]) => true; case _ => false }))
    println("p06 union-array-int-other " + ((Array(new Other): Any) match { case _: (Array[Box] | Array[Int]) => true; case _ => false }))
    println("p06 union-array-int-int " + ((Array(1): Any) match { case _: (Array[Box] | Array[Int]) => true; case _ => false }))
    println("p06 union-three " + (s match { case _: (Array[Box] | Array[Int] | String) => true; case _ => false }))

object P07_alias_inline:
  class Box(val n: Int)
  type AB = Array[Box]
  type Arr[T] = Array[T]
  inline def test(x: Any): Boolean = (x match { case _: (Array[Box]) => true; case _ => false })
  inline def testT[T](x: Any): Boolean = (x match { case _: (Array[T]) => true; case _ => false })
  transparent inline def testTr(x: Any): Boolean = (x match { case _: (Array[Box]) => true; case _ => false })
  inline def testAlias(x: Any): Boolean = (x match { case _: (AB) => true; case _ => false })
  def run(): Unit =
    val s: Any = Array("a")
    val b: Any = Array(new Box(1))
    println("p07 alias-unrelated " + (s match { case _: (AB) => true; case _ => false }))
    println("p07 alias-same " + (b match { case _: (AB) => true; case _ => false }))
    println("p07 alias-applied-unrelated " + (s match { case _: (Arr[Box]) => true; case _ => false }))
    println("p07 alias-applied-same " + (b match { case _: (Arr[Box]) => true; case _ => false }))
    println("p07 inline-unrelated " + test(s))
    println("p07 inline-same " + test(b))
    println("p07 inline-param-unrelated " + testT[Box](s))
    println("p07 inline-param-same " + testT[Box](b))
    println("p07 inline-param-int " + testT[Int](s))
    println("p07 transparent-unrelated " + testTr(s))
    println("p07 inline-alias-unrelated " + testAlias(s))
    println("p07 lambda-unrelated " + List(s).map({ case _: (Array[Box]) => true; case _ => false }))
    println("p07 lambda-same " + List(b).map({ case _: (Array[Box]) => true; case _ => false }))
    val f: Any => Boolean = { case _: (Array[Box]) => true; case _ => false }
    println("p07 function-value " + f(s) + " " + f(b))
    lazy val lz = (s match { case _: (Array[Box]) => true; case _ => false })
    println("p07 lazy " + lz)

object P13_checkable:
  class Box(val n: Int)
  class G[T](val x: T)
  def run(): Unit =
    val s: Any = Array("a")
    val l: Any = Array(List(1))
    val g: Any = Array(new G(1))
    val ls: Array[List[String]] = Array(List("a"))
    val ao: Array[Object] = Array(List("a"))
    println("p13 list-wild-from-any " + (l match { case _: (Array[List[?]]) => true; case _ => false }))
    println("p13 g-wild-from-any " + (g match { case _: (Array[G[?]]) => true; case _ => false }))

object P17_case_layers:
  trait Holder:
    type M = Array[Int] | Array[Array[String]]
    type N[T] = Array[Array[T]]
  object H extends Holder
  type TopU = Array[Int] | Array[Array[String]]
  def nested[T](x: Any): Boolean = (x match { case _: (Array[Array[T]] @unchecked) => true; case _ => false })
  def bound[T <: Array[Int]](x: Any): Boolean = (x match { case _: (Array[T] @unchecked) => true; case _ => false })
  def memberNested[T](x: Any): Boolean = (x match { case _: (H.N[T] @unchecked) => true; case _ => false })
  def unionGeneric[T](x: Any): Boolean = (x match { case _: ((Array[Array[T]] | Array[Int]) @unchecked) => true; case _ => false })
  def memberUnion(x: Any): Boolean = (x match { case _: (H.M) => true; case _ => false })
  def topUnion(x: Any): Boolean = (x match { case _: (TopU) => true; case _ => false })
  def wild2(x: Any): Boolean = (x match { case _: (Array[Array[?]]) => true; case _ => false })
  def any2(x: Any): Boolean = (x match { case _: (Array[Array[Any]]) => true; case _ => false })
  def three(x: Any): Boolean = (x match { case _: (Array[Array[Array[Int]]]) => true; case _ => false })
  def inter(x: Any): Boolean = (x match { case _: (Array[Int] & java.io.Serializable) => true; case _ => false })
  def run(): Unit =
    val inputs: List[(String, Any)] = List("s1" -> Array("a"), "s2" -> Array(Array("a")), "i1" -> Array(1), "i2" -> Array(Array(1)), "o1" -> Array[Any](Array(1), "x"), "i3" -> Array(Array(Array(1))), "str" -> "x")
    def row(name: String)(f: Any => Boolean): Unit = println(name + " " + inputs.map((n, v) => n + "=" + f(v)).mkString(" "))
    row("layers-nested")(nested[Int])
    row("layers-bound")(bound[Array[Int]])
    row("layers-member-nested")(memberNested[Int])
    row("layers-union-generic")(unionGeneric[Int])
    row("layers-member-union")(memberUnion)
    row("layers-top-union")(topUnion)
    row("layers-wild2")(wild2)
    row("layers-any2")(any2)
    row("layers-three")(three)
    row("layers-inter")(inter)

object R18_review:
  class Box(val n: Int)
  trait Tr extends Any
  class V(val x: Int) extends AnyVal with Tr
  class W2(val y: Int) extends AnyVal with Tr
  class TBox extends Tr
  def vNull(x: Any): Boolean = x.isInstanceOf[Array[V | Null]]
  def nullV(x: Any): Boolean = x.isInstanceOf[Array[Null | V]]
  def vNullUnion(x: Any): Boolean = x.isInstanceOf[Array[V | Null] | Array[Int]]
  def vTr(x: Any): Boolean = x.isInstanceOf[Array[V | Tr]]
  def vW(x: Any): Boolean = x.isInstanceOf[Array[V | W2]]
  def vBox(x: Any): Boolean = x.isInstanceOf[Array[V | TBox]]
  def vNothing(x: Any): Boolean = x.isInstanceOf[Array[V | Nothing]]
  def vInt(x: Any): Boolean = x.isInstanceOf[Array[V | Int]]
  def boxNull(x: Any): Boolean = x.isInstanceOf[Array[Box | Null]]
  def chain2(x: Any): Boolean = x.isInstanceOf[Array[Int *: String *: EmptyTuple] @unchecked]
  def chain1(x: Any): Boolean = x.isInstanceOf[Array[Int *: EmptyTuple] @unchecked]
  def chain3(x: Any): Boolean = x.isInstanceOf[Array[Int *: Int *: Int *: EmptyTuple] @unchecked]
  def chainNested(x: Any): Boolean = x.isInstanceOf[Array[(Int, String) *: EmptyTuple] @unchecked]
  def chainLayers(x: Any): Boolean = x.isInstanceOf[Array[Array[Int *: EmptyTuple]] @unchecked]
  inline def chainInline(x: Any): Boolean = x.isInstanceOf[Array[Int *: String *: EmptyTuple] @unchecked]
  def chainExpanded(x: Any): Boolean = chainInline(x)
  def refined(x: Any): Boolean = x.isInstanceOf[Array[Box { def n: Int }] @unchecked]
  def structural(x: Any): Boolean = x.isInstanceOf[Array[{ def n: Int }] @unchecked]
  def polyFunction(x: Any): Boolean = x.isInstanceOf[Array[[T] => T => T] @unchecked]
  def refinedLayers(x: Any): Boolean = x.isInstanceOf[Array[Array[Box { def n: Int }]] @unchecked]
  def refinedUnion(x: Any): Boolean = x.isInstanceOf[(Array[Box { def n: Int }] | Array[Int]) @unchecked]
  def run(): Unit =
    val inputs: List[(String, Any)] = List("strs" -> Array("a"), "ints" -> Array(1), "vs" -> Array(new V(1)), "ws" -> Array(new W2(1)), "trs" -> Array[Tr](new TBox), "objs" -> Array[Any](1), "boxes" -> Array(new Box(1)), "boxes2" -> Array(Array(new Box(1))), "strs2" -> Array(Array("a")), "ones" -> Array(Tuple1(1)), "pairs" -> Array((1, "a")), "ones2" -> Array(Array(Tuple1(1))), "fns" -> Array[Function1[Int, Int]](_ + 1), "str" -> "x")
    def row(name: String)(f: Any => Boolean): Unit = println(name + " " + inputs.map((n, v) => n + "=" + (if f(v) then 1 else 0)).mkString(" "))
    row("review-v-null")(vNull)
    row("review-null-v")(nullV)
    row("review-v-null-union")(vNullUnion)
    row("review-v-tr")(vTr)
    row("review-v-w")(vW)
    row("review-v-box")(vBox)
    row("review-v-nothing")(vNothing)
    row("review-v-int")(vInt)
    row("review-box-null")(boxNull)
    row("review-chain2")(chain2)
    row("review-chain1")(chain1)
    row("review-chain3")(chain3)
    row("review-chain-nested")(chainNested)
    row("review-chain-layers")(chainLayers)
    row("review-chain-inline")(chainExpanded)
    row("review-refined")(refined)
    row("review-structural")(structural)
    row("review-poly-function")(polyFunction)
    row("review-refined-layers")(refinedLayers)
    row("review-refined-union")(refinedUnion)

object P18_review:
  class Box(val n: Int)
  trait Tr extends Any
  class V(val x: Int) extends AnyVal with Tr
  class W2(val y: Int) extends AnyVal with Tr
  class TBox extends Tr
  def vNull(x: Any): Boolean = (x match { case _: (Array[V | Null]) => true; case _ => false })
  def nullV(x: Any): Boolean = (x match { case _: (Array[Null | V]) => true; case _ => false })
  def vNullUnion(x: Any): Boolean = (x match { case _: (Array[V | Null] | Array[Int]) => true; case _ => false })
  def vTr(x: Any): Boolean = (x match { case _: (Array[V | Tr]) => true; case _ => false })
  def vW(x: Any): Boolean = (x match { case _: (Array[V | W2]) => true; case _ => false })
  def vBox(x: Any): Boolean = (x match { case _: (Array[V | TBox]) => true; case _ => false })
  def vNothing(x: Any): Boolean = (x match { case _: (Array[V | Nothing]) => true; case _ => false })
  def vInt(x: Any): Boolean = (x match { case _: (Array[V | Int]) => true; case _ => false })
  def boxNull(x: Any): Boolean = (x match { case _: (Array[Box | Null]) => true; case _ => false })
  def chain2(x: Any): Boolean = (x match { case _: (Array[Int *: String *: EmptyTuple] @unchecked) => true; case _ => false })
  def chain1(x: Any): Boolean = (x match { case _: (Array[Int *: EmptyTuple] @unchecked) => true; case _ => false })
  def chain3(x: Any): Boolean = (x match { case _: (Array[Int *: Int *: Int *: EmptyTuple] @unchecked) => true; case _ => false })
  def chainNested(x: Any): Boolean = (x match { case _: (Array[(Int, String) *: EmptyTuple] @unchecked) => true; case _ => false })
  def chainLayers(x: Any): Boolean = (x match { case _: (Array[Array[Int *: EmptyTuple]] @unchecked) => true; case _ => false })
  inline def chainInline(x: Any): Boolean = (x match { case _: (Array[Int *: String *: EmptyTuple] @unchecked) => true; case _ => false })
  def chainExpanded(x: Any): Boolean = chainInline(x)
  def refined(x: Any): Boolean = (x match { case _: (Array[Box { def n: Int }] @unchecked) => true; case _ => false })
  def structural(x: Any): Boolean = (x match { case _: (Array[{ def n: Int }] @unchecked) => true; case _ => false })
  def polyFunction(x: Any): Boolean = (x match { case _: (Array[[T] => T => T] @unchecked) => true; case _ => false })
  def refinedLayers(x: Any): Boolean = (x match { case _: (Array[Array[Box { def n: Int }]] @unchecked) => true; case _ => false })
  def refinedUnion(x: Any): Boolean = (x match { case _: ((Array[Box { def n: Int }] | Array[Int]) @unchecked) => true; case _ => false })
  def run(): Unit =
    val inputs: List[(String, Any)] = List("strs" -> Array("a"), "ints" -> Array(1), "vs" -> Array(new V(1)), "ws" -> Array(new W2(1)), "trs" -> Array[Tr](new TBox), "objs" -> Array[Any](1), "boxes" -> Array(new Box(1)), "boxes2" -> Array(Array(new Box(1))), "strs2" -> Array(Array("a")), "ones" -> Array(Tuple1(1)), "pairs" -> Array((1, "a")), "ones2" -> Array(Array(Tuple1(1))), "fns" -> Array[Function1[Int, Int]](_ + 1), "str" -> "x")
    def row(name: String)(f: Any => Boolean): Unit = println(name + " " + inputs.map((n, v) => n + "=" + (if f(v) then 1 else 0)).mkString(" "))
    row("review-v-null")(vNull)
    row("review-null-v")(nullV)
    row("review-v-null-union")(vNullUnion)
    row("review-v-tr")(vTr)
    row("review-v-w")(vW)
    row("review-v-box")(vBox)
    row("review-v-nothing")(vNothing)
    row("review-v-int")(vInt)
    row("review-box-null")(boxNull)
    row("review-chain-layers")(chainLayers)
    row("review-refined")(refined)
    row("review-structural")(structural)
    row("review-poly-function")(polyFunction)
    row("review-refined-layers")(refinedLayers)
    row("review-refined-union")(refinedUnion)

object R19_universal:
  class Box(val n: Int)
  trait Tr extends Any
  trait Tr2
  class V(val x: Int) extends AnyVal with Tr
  class TBox extends Tr
  class Box2 extends Box(2) with Tr2
  def trNull(x: Any): Boolean = x.isInstanceOf[Array[Tr | Null]]
  def nullTr(x: Any): Boolean = x.isInstanceOf[Array[Null | Tr]]
  def trNullP(x: Any): Boolean = x match { case _: Array[Tr | Null] => true; case _ => false }
  def nullTrP(x: Any): Boolean = x match { case _: Array[Null | Tr] => true; case _ => false }
  def trNothing(x: Any): Boolean = x.isInstanceOf[Array[Tr | Nothing]]
  def trPlain(x: Any): Boolean = x.isInstanceOf[Array[Tr]]
  def vNullP(x: Any): Boolean = x match { case _: Array[V | Null] => true; case _ => false }
  def refNull(x: Any): Boolean = x.isInstanceOf[Array[Box { def n: Int } | Null] @unchecked]
  def refInter(x: Any): Boolean = x.isInstanceOf[Array[Box { def n: Int } & Tr2] @unchecked]
  def refUnion(x: Any): Boolean = x.isInstanceOf[Array[Box { def n: Int } | Tr2] @unchecked]
  def refNullP(x: Any): Boolean = x match { case _: (Array[Box { def n: Int } | Null] @unchecked) => true; case _ => false }
  def run(): Unit =
    val inputs: List[(String, Any)] = List("strs" -> Array("a"), "ints" -> Array(1), "vs" -> Array(new V(1)), "trs" -> Array[Tr](new TBox), "objs" -> Array[Any](1), "boxes" -> Array(new Box(1)), "boxes2" -> Array(new Box2), "tr2s" -> Array[Tr2](new Box2))
    def row(name: String)(f: Any => Boolean): Unit = println(name + " " + inputs.map((n, v) => n + "=" + (if f(v) then 1 else 0)).mkString(" "))
    row("tr-null")(trNull)
    row("null-tr")(nullTr)
    row("tr-null-pattern")(trNullP)
    row("null-tr-pattern")(nullTrP)
    row("tr-nothing")(trNothing)
    row("tr-plain")(trPlain)
    row("v-null-pattern")(vNullP)
    row("refined-null")(refNull)
    row("refined-inter")(refInter)
    row("refined-union")(refUnion)
    row("refined-null-pattern")(refNullP)

object R20_descriptors:
  trait Tr extends Any
  class V(val x: Int) extends AnyVal with Tr
  class WV[T](val w: T) extends AnyVal
  trait Gen[T]:
    def take(a: T): Int
    def give(): T
  class Impl extends Gen[Array[V | Null]]:
    def take(a: Array[V | Null]): Int = a.length
    def give(): Array[V | Null] = Holder.make(2)
  class Holder(val arr: Array[V | Null], val wv: WV[V | Null]):
    var field: Array[V | Null] = arr
    def param(a: Array[V | Null]): Int = a.length
    def result(): Array[V | Null] = field
    def wparam(w: WV[V | Null]): Boolean = w.w == null
    def wresult(): WV[V | Null] = wv
    def trParam(a: Array[Tr | Null]): Int = a.length
    def over(a: Array[V | Null]): String = "arr"
    def over(a: Array[Int]): String = "ints"
    def nested(a: Array[Array[V | Null]]): Int = a.length
  object Holder:
    def make(n: Int): Array[V | Null] = new Array[V | Null](n)
    def lam: Array[V | Null] => Int = a => a.length
  def members(c: Class[?], names: Set[String]): String = c.getDeclaredMethods.filter(m => names(m.getName)).map(_.toString).sorted.mkString("; ")
  def run(): Unit =
    val a: Array[V | Null] = Holder.make(3)
    a(0) = new V(7)
    val h = new Holder(a, new WV[V | Null](null))
    println("r20 sig-use " + h.param(a) + " " + h.result().length + " " + h.field.length + " " + h.wparam(new WV[V | Null](new V(2))) + " " + h.wparam(h.wresult()) + " " + h.over(a) + " " + h.over(Array(1)) + " " + h.nested(Array(a, a)))
    h.field = Holder.make(1)
    println("r20 sig-field-set " + h.field.length + " " + (a(1) == null) + " " + h.arr(0).asInstanceOf[V].x)
    val g: Gen[Array[V | Null]] = new Impl
    println("r20 sig-bridge " + g.take(a) + " " + g.give().length)
    val lam2: Array[V | Null] => Int = x => x.length
    println("r20 sig-lambda " + Holder.lam(a) + " " + lam2(a) + " " + List(a, h.result()).map(lam2).sum)
    println("r20 sig-universal " + h.trParam(new Array[Tr | Null](2)))
    println("r20 sig-members " + members(classOf[Holder], Set("param", "result", "wparam", "wresult", "trParam", "over", "nested", "arr", "wv", "field", "field_$eq")))
    println("r20 sig-bridges " + members(classOf[Impl], Set("take", "give")))

object R21_binder:
  trait Tr extends Any
  class V(val x: Int) extends AnyVal with Tr
  class Box(val n: Int)
  def bind(x: Any): String = x match { case a: Array[V | Null] => "arr" + a.length; case _ => "no" }
  def bindElem(x: Any): String = x match { case a: Array[V | Null] => "elem " + (a.length > 0 && a(0) != null); case _ => "no" }
  def testThenCast(x: Any): String = if x.isInstanceOf[Array[V | Null]] then "len" + x.asInstanceOf[Array[V | Null]].length else "no"
  def pass(a: Array[V | Null]): Int = a.length
  def bindPass(x: Any): String = x match { case a: Array[V | Null] => "pass" + pass(a); case _ => "no" }
  def refined(x: Any): String = x match { case a: (Array[Box { def n: Int }] @unchecked) => "arr" + a.length + (if a.length > 0 then " " + a(0).n else ""); case _ => "no" }
  def chain(x: Any): String = if x.isInstanceOf[Array[Int *: EmptyTuple] @unchecked] then "len" + x.asInstanceOf[Array[Int *: EmptyTuple]].length else "no"
  def run(): Unit =
    val inputs: List[(String, Any)] = List("strs" -> Array("a"), "ints" -> Array(1), "vs" -> Array(new V(1)), "objs" -> Array[Any](1), "boxes" -> Array(new Box(1)), "pairs" -> Array((1, "a")), "ones" -> Array(Tuple1(1)))
    def row(name: String)(f: Any => String): Unit = println(name + " " + inputs.map((n, v) => n + "=" + f(v)).mkString(" "))
    row("bind")(bind)
    row("bind-elem")(bindElem)
    row("test-then-cast")(testThenCast)
    row("bind-pass")(bindPass)
    row("refined")(refined)
    row("chain")(chain)

@main def run(): Unit =
  R01_kinds.run()
  R02_static.run()
  R02B_serializable.run()
  R04_nested.run()
  R05_special.run()
  R06_union_inter.run()
  R07_alias_inline.run()
  R12_class_member.run()
  R13_checkable.run()
  R17_case_layers.run()
  P01_kinds.run()
  P04_nested.run()
  P06_union_inter.run()
  P07_alias_inline.run()
  P13_checkable.run()
  P17_case_layers.run()
  R18_review.run()
  P18_review.run()
  R19_universal.run()
  R20_descriptors.run()
  R21_binder.run()
