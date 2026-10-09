//> using platform js
//> using dep org.scala-js:scalajs-test-interface_2.13:1.20.1
// sbt's test interface on Scala.js: `Framework` carries `@EnableReflectiveInstantiation`, so a
// class implementing it is found by name, as the test bridge finds a framework, and a class
// that does not is not.
import scala.scalajs.reflect.Reflect
import sbt.testing.*

class MyFramework extends Framework:
  def name(): String = "mine"
  def fingerprints(): Array[Fingerprint] = Array(new SubclassFingerprint {
    def isModule(): Boolean = false
    def superclassName(): String = "MySuite"
    def requireNoArgConstructor(): Boolean = true
  })
  def runner(args: Array[String], remoteArgs: Array[String], testClassLoader: ClassLoader): Runner = null
  def slaveRunner(args: Array[String], remoteArgs: Array[String], testClassLoader: ClassLoader, send: String => Unit): Runner = null

class NoFramework

@main def run(): Unit =
  val found = Reflect.lookupInstantiatableClass("MyFramework")
  println(found.isDefined)
  val framework = found.get.newInstance().asInstanceOf[Framework]
  println(framework.name())
  println(framework.fingerprints().map { case f: SubclassFingerprint => f.superclassName() }.toList)
  println(classOf[Framework].isAssignableFrom(found.get.runtimeClass))
  println(Reflect.lookupInstantiatableClass("NoFramework").isDefined)
  println(Status.Failure.name() + " " + Status.Failure.ordinal() + " " + Status.values().length)
  val task = new TaskDef("a.B", framework.fingerprints()(0), false, Array(new SuiteSelector, new TestSelector("t")))
  println(task.fullyQualifiedName() + " " + task.selectors().toList + " " + task.explicitlySpecified())
  println(task == new TaskDef("a.B", framework.fingerprints()(0), false, Array(new SuiteSelector, new TestSelector("t"))))
  println(new OptionalThrowable(new RuntimeException("boom")).get().getMessage + " " + new OptionalThrowable().isEmpty())
  println(Status.valueOf("Skipped").ordinal())
