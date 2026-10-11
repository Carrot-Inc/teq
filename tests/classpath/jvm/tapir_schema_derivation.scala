// jars: scala-library tapir-core-jvm magnolia123-jvm sttp-model-core-jvm sttp-shared-core-jvm sttp-shared-ws-jvm
// std: scala-library
//> using dep com.softwaremill.sttp.tapir::tapir-core:1.13.29
// tapir's automatic `Schema` derivation on the JVM: its inline body makes an anonymous `Derivation`, whose
// copy at each expansion keeps the by-name parameters of its members (magnolia passes the type classes
// as thunks), the thunks' nodes no values of their own (dotty's `ExprType` through `TreeTypeMap`).
import sttp.tapir.Schema
import sttp.tapir.generic.auto.*
case class Inner(code: String, n: Option[Int])
case class Person(name: String, age: Int, inner: Inner, tags: List[String])
@main def run(): Unit =
  val s = summon[Schema[Person]]
  println(s.name.map(_.fullName))
  println(s.schemaType.getClass.getSimpleName)
  val d = Schema.derived[Inner]
  println(d.name.map(_.fullName))
