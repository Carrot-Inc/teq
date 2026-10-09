// jars: scala-library cats-kernel-sjs cats-core-sjs alleycats-core shapeless3-deriving kittens
// Thirty case classes of 3 to 8 fields over strings, numbers, options, lists and a nested model,
// each `derives Eq, Show, Order` through kittens from its jar (shapeless3's inline instances
// expanded by the typer) and each shown, compared and ordered once; the `kittens30` row of
// bench/budget.sh. Its output is scalac 3.8.4's.
//> using dep org.typelevel::cats-core:2.13.0
//> using dep org.typelevel::kittens:3.5.0
import cats.*
import cats.derived.*
import cats.syntax.all.*

case class M0(f0: String, f1: Int, f2: Boolean) derives Eq, Show, Order
case class M1(f0: Int, f1: Boolean, f2: Option[String], f3: List[Int]) derives Eq, Show, Order
case class M2(f0: Boolean, f1: Option[String], f2: List[Int], f3: Double, f4: Long, ref: M1) derives Eq, Show, Order
case class M3(f0: Option[String], f1: List[Int], f2: Double, f3: Long, f4: String, f5: Int) derives Eq, Show, Order
case class M4(f0: List[Int], f1: Double, f2: Long, f3: String, f4: Int, f5: Boolean, f6: Option[String]) derives Eq, Show, Order
case class M5(f0: Double, f1: Long, f2: String, f3: Int, f4: Boolean, f5: Option[String], f6: List[Int], f7: Double, ref: M4) derives Eq, Show, Order
case class M6(f0: Long, f1: String, f2: Int) derives Eq, Show, Order
case class M7(f0: String, f1: Int, f2: Boolean, f3: Option[String]) derives Eq, Show, Order
case class M8(f0: Int, f1: Boolean, f2: Option[String], f3: List[Int], f4: Double, ref: M7) derives Eq, Show, Order
case class M9(f0: Boolean, f1: Option[String], f2: List[Int], f3: Double, f4: Long, f5: String) derives Eq, Show, Order
case class M10(f0: Option[String], f1: List[Int], f2: Double, f3: Long, f4: String, f5: Int, f6: Boolean) derives Eq, Show, Order
case class M11(f0: List[Int], f1: Double, f2: Long, f3: String, f4: Int, f5: Boolean, f6: Option[String], f7: List[Int], ref: M10) derives Eq, Show, Order
case class M12(f0: Double, f1: Long, f2: String) derives Eq, Show, Order
case class M13(f0: Long, f1: String, f2: Int, f3: Boolean) derives Eq, Show, Order
case class M14(f0: String, f1: Int, f2: Boolean, f3: Option[String], f4: List[Int], ref: M13) derives Eq, Show, Order
case class M15(f0: Int, f1: Boolean, f2: Option[String], f3: List[Int], f4: Double, f5: Long) derives Eq, Show, Order
case class M16(f0: Boolean, f1: Option[String], f2: List[Int], f3: Double, f4: Long, f5: String, f6: Int) derives Eq, Show, Order
case class M17(f0: Option[String], f1: List[Int], f2: Double, f3: Long, f4: String, f5: Int, f6: Boolean, f7: Option[String], ref: M16) derives Eq, Show, Order
case class M18(f0: List[Int], f1: Double, f2: Long) derives Eq, Show, Order
case class M19(f0: Double, f1: Long, f2: String, f3: Int) derives Eq, Show, Order
case class M20(f0: Long, f1: String, f2: Int, f3: Boolean, f4: Option[String], ref: M19) derives Eq, Show, Order
case class M21(f0: String, f1: Int, f2: Boolean, f3: Option[String], f4: List[Int], f5: Double) derives Eq, Show, Order
case class M22(f0: Int, f1: Boolean, f2: Option[String], f3: List[Int], f4: Double, f5: Long, f6: String) derives Eq, Show, Order
case class M23(f0: Boolean, f1: Option[String], f2: List[Int], f3: Double, f4: Long, f5: String, f6: Int, f7: Boolean, ref: M22) derives Eq, Show, Order
case class M24(f0: Option[String], f1: List[Int], f2: Double) derives Eq, Show, Order
case class M25(f0: List[Int], f1: Double, f2: Long, f3: String) derives Eq, Show, Order
case class M26(f0: Double, f1: Long, f2: String, f3: Int, f4: Boolean, ref: M25) derives Eq, Show, Order
case class M27(f0: Long, f1: String, f2: Int, f3: Boolean, f4: Option[String], f5: List[Int]) derives Eq, Show, Order
case class M28(f0: String, f1: Int, f2: Boolean, f3: Option[String], f4: List[Int], f5: Double, f6: Long) derives Eq, Show, Order
case class M29(f0: Int, f1: Boolean, f2: Option[String], f3: List[Int], f4: Double, f5: Long, f6: String, f7: Int, ref: M28) derives Eq, Show, Order

object Main:
  def p(xs: Any*): Unit = println(xs.mkString(" "))
  def main(args: Array[String]): Unit =
    val m0 = M0("s1", 1, true)
    val m1 = M1(2, true, Some("o2"), List(2, 2))
    val m2 = M2(true, Some("o3"), List(3, 2), 3.5, 3L, m1)
    val m3 = M3(Some("o4"), List(4, 2), 4.5, 4L, "s4", 4)
    val m4 = M4(List(5, 2), 5.5, 5L, "s5", 5, true, Some("o5"))
    val m5 = M5(6.5, 6L, "s6", 6, true, Some("o6"), List(6, 2), 6.5, m4)
    val m6 = M6(7L, "s7", 7)
    val m7 = M7("s8", 8, true, Some("o8"))
    val m8 = M8(9, true, Some("o9"), List(9, 2), 9.5, m7)
    val m9 = M9(true, Some("o10"), List(10, 2), 10.5, 10L, "s10")
    val m10 = M10(Some("o11"), List(11, 2), 11.5, 11L, "s11", 11, true)
    val m11 = M11(List(12, 2), 12.5, 12L, "s12", 12, true, Some("o12"), List(12, 2), m10)
    val m12 = M12(13.5, 13L, "s13")
    val m13 = M13(14L, "s14", 14, true)
    val m14 = M14("s15", 15, true, Some("o15"), List(15, 2), m13)
    val m15 = M15(16, true, Some("o16"), List(16, 2), 16.5, 16L)
    val m16 = M16(true, Some("o17"), List(17, 2), 17.5, 17L, "s17", 17)
    val m17 = M17(Some("o18"), List(18, 2), 18.5, 18L, "s18", 18, true, Some("o18"), m16)
    val m18 = M18(List(19, 2), 19.5, 19L)
    val m19 = M19(20.5, 20L, "s20", 20)
    val m20 = M20(21L, "s21", 21, true, Some("o21"), m19)
    val m21 = M21("s22", 22, true, Some("o22"), List(22, 2), 22.5)
    val m22 = M22(23, true, Some("o23"), List(23, 2), 23.5, 23L, "s23")
    val m23 = M23(true, Some("o24"), List(24, 2), 24.5, 24L, "s24", 24, true, m22)
    val m24 = M24(Some("o25"), List(25, 2), 25.5)
    val m25 = M25(List(26, 2), 26.5, 26L, "s26")
    val m26 = M26(27.5, 27L, "s27", 27, true, m25)
    val m27 = M27(28L, "s28", 28, true, Some("o28"), List(28, 2))
    val m28 = M28("s29", 29, true, Some("o29"), List(29, 2), 29.5, 29L)
    val m29 = M29(30, true, Some("o30"), List(30, 2), 30.5, 30L, "s30", 30, m28)
    p(m0.show, m0 === m0, Order[M0].compare(m0, m0), m0 < m0)
    p(m1.show, m1 === m1, Order[M1].compare(m1, m1), m1 < m1)
    p(m2.show, m2 === m2, Order[M2].compare(m2, m2), m2 < m2)
    p(m3.show, m3 === m3, Order[M3].compare(m3, m3), m3 < m3)
    p(m4.show, m4 === m4, Order[M4].compare(m4, m4), m4 < m4)
    p(m5.show, m5 === m5, Order[M5].compare(m5, m5), m5 < m5)
    p(m6.show, m6 === m6, Order[M6].compare(m6, m6), m6 < m6)
    p(m7.show, m7 === m7, Order[M7].compare(m7, m7), m7 < m7)
    p(m8.show, m8 === m8, Order[M8].compare(m8, m8), m8 < m8)
    p(m9.show, m9 === m9, Order[M9].compare(m9, m9), m9 < m9)
    p(m10.show, m10 === m10, Order[M10].compare(m10, m10), m10 < m10)
    p(m11.show, m11 === m11, Order[M11].compare(m11, m11), m11 < m11)
    p(m12.show, m12 === m12, Order[M12].compare(m12, m12), m12 < m12)
    p(m13.show, m13 === m13, Order[M13].compare(m13, m13), m13 < m13)
    p(m14.show, m14 === m14, Order[M14].compare(m14, m14), m14 < m14)
    p(m15.show, m15 === m15, Order[M15].compare(m15, m15), m15 < m15)
    p(m16.show, m16 === m16, Order[M16].compare(m16, m16), m16 < m16)
    p(m17.show, m17 === m17, Order[M17].compare(m17, m17), m17 < m17)
    p(m18.show, m18 === m18, Order[M18].compare(m18, m18), m18 < m18)
    p(m19.show, m19 === m19, Order[M19].compare(m19, m19), m19 < m19)
    p(m20.show, m20 === m20, Order[M20].compare(m20, m20), m20 < m20)
    p(m21.show, m21 === m21, Order[M21].compare(m21, m21), m21 < m21)
    p(m22.show, m22 === m22, Order[M22].compare(m22, m22), m22 < m22)
    p(m23.show, m23 === m23, Order[M23].compare(m23, m23), m23 < m23)
    p(m24.show, m24 === m24, Order[M24].compare(m24, m24), m24 < m24)
    p(m25.show, m25 === m25, Order[M25].compare(m25, m25), m25 < m25)
    p(m26.show, m26 === m26, Order[M26].compare(m26, m26), m26 < m26)
    p(m27.show, m27 === m27, Order[M27].compare(m27, m27), m27 < m27)
    p(m28.show, m28 === m28, Order[M28].compare(m28, m28), m28 < m28)
    p(m29.show, m29 === m29, Order[M29].compare(m29, m29), m29 < m29)
