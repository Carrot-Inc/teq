// jars: scala-library tapir-core sttp-model sttp-shared-core sttp-shared-ws magnolia123 scala-java-time
// The thirty models of zio30.scala, each `derives Schema` from the tapir jar (Magnolia's derivation
// with tapir's `SchemaMagnoliaDerivation` running in the interpreter) and each schema named once;
// the `schema30` row of bench/budget.sh.
import sttp.tapir.Schema

case class M0(f0: String, f1: Int, f2: Boolean) derives Schema
case class M1(f0: Int, f1: Boolean, f2: Option[String], f3: List[Int]) derives Schema
case class M2(f0: Boolean, f1: Option[String], f2: List[Int], f3: Double, f4: Long) derives Schema
case class M3(f0: Option[String], f1: List[Int], f2: Double, f3: Long, f4: String, f5: Int, ref: M2) derives Schema
case class M4(f0: List[Int], f1: Double, f2: Long, f3: String, f4: Int, f5: Boolean, f6: Option[String]) derives Schema
case class M5(f0: Double, f1: Long, f2: String) derives Schema
case class M6(f0: Long, f1: String, f2: Int, f3: Boolean, ref: M5) derives Schema
case class M7(f0: String, f1: Int, f2: Boolean, f3: Option[String], f4: List[Int]) derives Schema
case class M8(f0: Int, f1: Boolean, f2: Option[String], f3: List[Int], f4: Double, f5: Long) derives Schema
case class M9(f0: Boolean, f1: Option[String], f2: List[Int], f3: Double, f4: Long, f5: String, f6: Int, ref: M8) derives Schema
case class M10(f0: Option[String], f1: List[Int], f2: Double) derives Schema
case class M11(f0: List[Int], f1: Double, f2: Long, f3: String) derives Schema
case class M12(f0: Double, f1: Long, f2: String, f3: Int, f4: Boolean, ref: M11) derives Schema
case class M13(f0: Long, f1: String, f2: Int, f3: Boolean, f4: Option[String], f5: List[Int]) derives Schema
case class M14(f0: String, f1: Int, f2: Boolean, f3: Option[String], f4: List[Int], f5: Double, f6: Long) derives Schema
case class M15(f0: Int, f1: Boolean, f2: Option[String], ref: M14) derives Schema
case class M16(f0: Boolean, f1: Option[String], f2: List[Int], f3: Double) derives Schema
case class M17(f0: Option[String], f1: List[Int], f2: Double, f3: Long, f4: String) derives Schema
case class M18(f0: List[Int], f1: Double, f2: Long, f3: String, f4: Int, f5: Boolean, ref: M17) derives Schema
case class M19(f0: Double, f1: Long, f2: String, f3: Int, f4: Boolean, f5: Option[String], f6: List[Int]) derives Schema
case class M20(f0: Long, f1: String, f2: Int) derives Schema
case class M21(f0: String, f1: Int, f2: Boolean, f3: Option[String], ref: M20) derives Schema
case class M22(f0: Int, f1: Boolean, f2: Option[String], f3: List[Int], f4: Double) derives Schema
case class M23(f0: Boolean, f1: Option[String], f2: List[Int], f3: Double, f4: Long, f5: String) derives Schema
case class M24(f0: Option[String], f1: List[Int], f2: Double, f3: Long, f4: String, f5: Int, f6: Boolean, ref: M23) derives Schema
case class M25(f0: List[Int], f1: Double, f2: Long) derives Schema
case class M26(f0: Double, f1: Long, f2: String, f3: Int) derives Schema
case class M27(f0: Long, f1: String, f2: Int, f3: Boolean, f4: Option[String], ref: M26) derives Schema
case class M28(f0: String, f1: Int, f2: Boolean, f3: Option[String], f4: List[Int], f5: Double) derives Schema
case class M29(f0: Int, f1: Boolean, f2: Option[String], f3: List[Int], f4: Double, f5: Long, f6: String) derives Schema
@main def main(): Unit =
  println(summon[Schema[M0]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M0]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M1]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M1]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M2]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M2]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M3]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M3]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M4]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M4]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M5]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M5]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M6]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M6]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M7]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M7]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M8]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M8]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M9]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M9]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M10]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M10]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M11]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M11]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M12]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M12]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M13]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M13]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M14]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M14]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M15]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M15]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M16]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M16]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M17]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M17]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M18]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M18]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M19]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M19]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M20]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M20]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M21]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M21]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M22]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M22]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M23]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M23]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M24]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M24]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M25]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M25]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M26]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M26]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M27]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M27]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M28]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M28]].schemaType.getClass.getSimpleName)
  println(summon[Schema[M29]].name.map(_.fullName).getOrElse("?") + " " + summon[Schema[M29]].schemaType.getClass.getSimpleName)
