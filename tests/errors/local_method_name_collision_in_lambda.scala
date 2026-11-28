object Probe:
  def main(args: Array[String]): Unit =
    val f = () =>
      locally {
        object Values:
          def count$1(): Int = 1
          def read(): Int =
            def count(): Int = 2
            count()
        Values.read()
      }
    f()

// A local method and an explicitly declared member have the same name and descriptor after lifting.
// An enclosing closure contains the expression.
// scalac 3.8.4 builds it.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
