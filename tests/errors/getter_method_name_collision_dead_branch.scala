object Probe:
  def main(args: Array[String]): Unit =
    if false then
      locally {
        import scala.annotation.targetName
        class Count:
          val x = 1
          @targetName("x") def read(): Int = 2
        new Count().read()
      }
    else ()

// A method's target name and signature coincide with a val's getter. It appears in the untaken
// branch of a constant conditional.
// scalac 3.8.4 builds it.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Count$18vinar_145.read: internal error: a second method x()I in the class
// expect: 1 construct not supported by the JVM backend
