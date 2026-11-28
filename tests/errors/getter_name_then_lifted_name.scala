object Probe:
  def main(args: Array[String]): Unit =
    locally {
      import scala.annotation.targetName
      class Count:
        val x = 1
        @targetName("x") def read(): Int = 2
      new Count().read()
    }
    locally {
      object Values:
        def count$1(): Int = 1
        def read(): Int =
          def count(): Int = 2
          count()
      Values.read()
    }

// A method's target name and signature coincide with a val's getter. Next, a local method and an
// explicitly declared member have the same name and descriptor after lifting.
// scalac 3.8.4 rejects it: Conflicting definitions:
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Count$1c5q0dz_121.read: internal error: a second method x()I in the class
// expect: 1 construct not supported by the JVM backend
