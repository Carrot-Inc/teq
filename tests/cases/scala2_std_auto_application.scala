//> using platform js
//> using dep org.scala-js:scalajs-test-interface_2.13:1.20.1
// interp-expected: js
// sbt's test interface is a Scala 2 artifact, whose `def f()` a Scala 3 caller may call as `f`;
// so may it the std's port of it.
import sbt.testing.*

@main def run =
  val fingerprint = new SubclassFingerprint:
    def isModule(): Boolean = true
    def superclassName(): String = "p.Spec"
    def requireNoArgConstructor(): Boolean = false
  val task = new TaskDef("p.MySpec", fingerprint, false, Array(new SuiteSelector))
  println(task.fullyQualifiedName)
  println(task.explicitlySpecified)
  println(task.selectors.length)
  println(fingerprint.isModule)
  println(task.fullyQualifiedName())
