// jars: zio-test-sbt zio-test zio zio-streams zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat macrotask-executor scala-java-time
//> using platform js
//> using dep dev.zio::zio-test-sbt::2.1.26
// A framework of a jar found by name at run time, as the test bridge finds it: its class
// inherits `@EnableReflectiveInstantiation` from sbt's `Framework`, a trait of the std, which
// the jar's TASTy names in the annotation's place. The expectation is Scala.js 1.21's
// (`scala-cli run --js-version 1.21.0`: zio-test-sbt's IR is too new for scala-cli's default).
import scala.scalajs.reflect.Reflect

@main def run(): Unit =
  val found = Reflect.lookupInstantiatableClass("zio.test.sbt.ZTestFramework")
  println(found.isDefined)
  println(found.exists(c => classOf[sbt.testing.Framework].isAssignableFrom(c.runtimeClass)))
  val framework = found.get.newInstance().asInstanceOf[sbt.testing.Framework]
  println(framework.fingerprints().length)
