//> using platform jvm
// Erasure walks a type to its end: a refinement of 33 members, a chain of 33 opaque types and an array of 34
// dimensions each erase as their underlying class, for a ClassTag and for a checked cast, however deep they are.
import scala.reflect.ClassTag
class C
object O:
  opaque type O1 = String
  opaque type O2 = O1
  opaque type O3 = O2
  opaque type O4 = O3
  opaque type O5 = O4
  opaque type O6 = O5
  opaque type O7 = O6
  opaque type O8 = O7
  opaque type O9 = O8
  opaque type O10 = O9
  opaque type O11 = O10
  opaque type O12 = O11
  opaque type O13 = O12
  opaque type O14 = O13
  opaque type O15 = O14
  opaque type O16 = O15
  opaque type O17 = O16
  opaque type O18 = O17
  opaque type O19 = O18
  opaque type O20 = O19
  opaque type O21 = O20
  opaque type O22 = O21
  opaque type O23 = O22
  opaque type O24 = O23
  opaque type O25 = O24
  opaque type O26 = O25
  opaque type O27 = O26
  opaque type O28 = O27
  opaque type O29 = O28
  opaque type O30 = O29
  opaque type O31 = O30
  opaque type O32 = O31
  opaque type O33 = O32
object Main:
  type R = C { def m1: Int; def m2: Int; def m3: Int; def m4: Int; def m5: Int; def m6: Int; def m7: Int; def m8: Int; def m9: Int; def m10: Int; def m11: Int; def m12: Int; def m13: Int; def m14: Int; def m15: Int; def m16: Int; def m17: Int; def m18: Int; def m19: Int; def m20: Int; def m21: Int; def m22: Int; def m23: Int; def m24: Int; def m25: Int; def m26: Int; def m27: Int; def m28: Int; def m29: Int; def m30: Int; def m31: Int; def m32: Int; def m33: Int }
  def cast[T](x: Any)(using ClassTag[T]): String = x match
    case _: T => "matched"
    case _ => "not matched"
  def main(args: Array[String]): Unit =
    println(summon[ClassTag[R]].runtimeClass.getName)
    println(summon[ClassTag[O.O33]].runtimeClass.getName)
    println(try { "a".asInstanceOf[O.O33]; "cast" } catch { case _: ClassCastException => "CCE" })
    println(try { 1.asInstanceOf[O.O33]; "cast" } catch { case _: ClassCastException => "CCE" })
    println(try { "s".asInstanceOf[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[Array[String]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]].length.toString } catch { case _: ClassCastException => "CCE" })
