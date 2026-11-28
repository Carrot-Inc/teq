object Probe:
  def main(args: Array[String]): Unit =
    if false then
      locally {
        object Values:
          def count$1(): Int = 1
          def read(): Int =
            def count(): Int = 2
            count()
        Values.read()
      }
    else ()

// A local method and an explicitly declared member have the same name and descriptor after lifting.
// It appears in the untaken branch of a constant conditional.
// scalac 3.8.4 builds it.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 0
