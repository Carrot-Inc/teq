// teq: --target jvm --std=scala-library
// A `finalize` overrides java.lang.Object's on the JVM: scalac accepts
// it (DisableSyntax's noFinalize fixtures, tests/fix/REFERENCES.md).
class Resource { override def finalize(): Unit = println("exit") }
@main def run(): Unit = println(new Resource() != null)
