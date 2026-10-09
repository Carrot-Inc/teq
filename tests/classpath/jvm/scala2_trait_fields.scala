// jars: scala-library scala2-lib
// A class mixing in a Scala 2.13 jar's trait with a private val, whose member the transcoded pickle leaves out:
// the trait's class file alone gives the class the field, the getter and the setter `T$_setter_$x_$eq` that the
// trait's `$init$` stores through (`class_file_storage`), as master's class-file rule did.
class S extends scala2lib.Settings
object SO extends scala2lib.Settings

@main def run(): Unit =
  println(S().name + " " + S().reveal + " " + SO.reveal)
