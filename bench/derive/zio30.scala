// jars: scala-library zio-json magnolia zio
// Thirty case classes of 3 to 8 fields over strings, numbers, options, lists and a nested model,
// each `derives JsonCodec` from the zio-json jar (Magnolia running in the interpreter) and each
// encoded once; the `derive30` row of bench/budget.sh. Its output is scalac 3.8.4's.
import zio.json.*

case class M0(f0: String, f1: Int, f2: Boolean) derives JsonCodec
case class M1(f0: Int, f1: Boolean, f2: Option[String], f3: List[Int]) derives JsonCodec
case class M2(f0: Boolean, f1: Option[String], f2: List[Int], f3: Double, f4: Long) derives JsonCodec
case class M3(f0: Option[String], f1: List[Int], f2: Double, f3: Long, f4: String, f5: Int, ref: M2) derives JsonCodec
case class M4(f0: List[Int], f1: Double, f2: Long, f3: String, f4: Int, f5: Boolean, f6: Option[String]) derives JsonCodec
case class M5(f0: Double, f1: Long, f2: String) derives JsonCodec
case class M6(f0: Long, f1: String, f2: Int, f3: Boolean, ref: M5) derives JsonCodec
case class M7(f0: String, f1: Int, f2: Boolean, f3: Option[String], f4: List[Int]) derives JsonCodec
case class M8(f0: Int, f1: Boolean, f2: Option[String], f3: List[Int], f4: Double, f5: Long) derives JsonCodec
case class M9(f0: Boolean, f1: Option[String], f2: List[Int], f3: Double, f4: Long, f5: String, f6: Int, ref: M8) derives JsonCodec
case class M10(f0: Option[String], f1: List[Int], f2: Double) derives JsonCodec
case class M11(f0: List[Int], f1: Double, f2: Long, f3: String) derives JsonCodec
case class M12(f0: Double, f1: Long, f2: String, f3: Int, f4: Boolean, ref: M11) derives JsonCodec
case class M13(f0: Long, f1: String, f2: Int, f3: Boolean, f4: Option[String], f5: List[Int]) derives JsonCodec
case class M14(f0: String, f1: Int, f2: Boolean, f3: Option[String], f4: List[Int], f5: Double, f6: Long) derives JsonCodec
case class M15(f0: Int, f1: Boolean, f2: Option[String], ref: M14) derives JsonCodec
case class M16(f0: Boolean, f1: Option[String], f2: List[Int], f3: Double) derives JsonCodec
case class M17(f0: Option[String], f1: List[Int], f2: Double, f3: Long, f4: String) derives JsonCodec
case class M18(f0: List[Int], f1: Double, f2: Long, f3: String, f4: Int, f5: Boolean, ref: M17) derives JsonCodec
case class M19(f0: Double, f1: Long, f2: String, f3: Int, f4: Boolean, f5: Option[String], f6: List[Int]) derives JsonCodec
case class M20(f0: Long, f1: String, f2: Int) derives JsonCodec
case class M21(f0: String, f1: Int, f2: Boolean, f3: Option[String], ref: M20) derives JsonCodec
case class M22(f0: Int, f1: Boolean, f2: Option[String], f3: List[Int], f4: Double) derives JsonCodec
case class M23(f0: Boolean, f1: Option[String], f2: List[Int], f3: Double, f4: Long, f5: String) derives JsonCodec
case class M24(f0: Option[String], f1: List[Int], f2: Double, f3: Long, f4: String, f5: Int, f6: Boolean, ref: M23) derives JsonCodec
case class M25(f0: List[Int], f1: Double, f2: Long) derives JsonCodec
case class M26(f0: Double, f1: Long, f2: String, f3: Int) derives JsonCodec
case class M27(f0: Long, f1: String, f2: Int, f3: Boolean, f4: Option[String], ref: M26) derives JsonCodec
case class M28(f0: String, f1: Int, f2: Boolean, f3: Option[String], f4: List[Int], f5: Double) derives JsonCodec
case class M29(f0: Int, f1: Boolean, f2: Option[String], f3: List[Int], f4: Double, f5: Long, f6: String) derives JsonCodec
@main def main(): Unit =
  val M0Value = M0("s0", 0, true)
  val M1Value = M1(1, true, Some("o"), List(1, 2))
  val M2Value = M2(true, Some("o"), List(1, 2), 1.5, 7L)
  val M3Value = M3(Some("o"), List(1, 2), 1.5, 7L, "s3", 3, M2Value)
  val M4Value = M4(List(1, 2), 1.5, 7L, "s4", 4, true, Some("o"))
  val M5Value = M5(1.5, 7L, "s5")
  val M6Value = M6(7L, "s6", 6, true, M5Value)
  val M7Value = M7("s7", 7, true, Some("o"), List(1, 2))
  val M8Value = M8(8, true, Some("o"), List(1, 2), 1.5, 7L)
  val M9Value = M9(true, Some("o"), List(1, 2), 1.5, 7L, "s9", 9, M8Value)
  val M10Value = M10(Some("o"), List(1, 2), 1.5)
  val M11Value = M11(List(1, 2), 1.5, 7L, "s11")
  val M12Value = M12(1.5, 7L, "s12", 12, true, M11Value)
  val M13Value = M13(7L, "s13", 13, true, Some("o"), List(1, 2))
  val M14Value = M14("s14", 14, true, Some("o"), List(1, 2), 1.5, 7L)
  val M15Value = M15(15, true, Some("o"), M14Value)
  val M16Value = M16(true, Some("o"), List(1, 2), 1.5)
  val M17Value = M17(Some("o"), List(1, 2), 1.5, 7L, "s17")
  val M18Value = M18(List(1, 2), 1.5, 7L, "s18", 18, true, M17Value)
  val M19Value = M19(1.5, 7L, "s19", 19, true, Some("o"), List(1, 2))
  val M20Value = M20(7L, "s20", 20)
  val M21Value = M21("s21", 21, true, Some("o"), M20Value)
  val M22Value = M22(22, true, Some("o"), List(1, 2), 1.5)
  val M23Value = M23(true, Some("o"), List(1, 2), 1.5, 7L, "s23")
  val M24Value = M24(Some("o"), List(1, 2), 1.5, 7L, "s24", 24, true, M23Value)
  val M25Value = M25(List(1, 2), 1.5, 7L)
  val M26Value = M26(1.5, 7L, "s26", 26)
  val M27Value = M27(7L, "s27", 27, true, Some("o"), M26Value)
  val M28Value = M28("s28", 28, true, Some("o"), List(1, 2), 1.5)
  val M29Value = M29(29, true, Some("o"), List(1, 2), 1.5, 7L, "s29")
  println(M0Value.toJson)
  println(M1Value.toJson)
  println(M2Value.toJson)
  println(M3Value.toJson)
  println(M4Value.toJson)
  println(M5Value.toJson)
  println(M6Value.toJson)
  println(M7Value.toJson)
  println(M8Value.toJson)
  println(M9Value.toJson)
  println(M10Value.toJson)
  println(M11Value.toJson)
  println(M12Value.toJson)
  println(M13Value.toJson)
  println(M14Value.toJson)
  println(M15Value.toJson)
  println(M16Value.toJson)
  println(M17Value.toJson)
  println(M18Value.toJson)
  println(M19Value.toJson)
  println(M20Value.toJson)
  println(M21Value.toJson)
  println(M22Value.toJson)
  println(M23Value.toJson)
  println(M24Value.toJson)
  println(M25Value.toJson)
  println(M26Value.toJson)
  println(M27Value.toJson)
  println(M28Value.toJson)
  println(M29Value.toJson)

def roundTrip(): Unit = ()

