// teq: --std=scala-library
// command: build
// expect: not supported on JavaScript: class AbstractExecutorService java.util.concurrent.AbstractExecutorService (a Java definition without an implementation in the std's platform layer)
// scala-library's `ExecutionContext.global` reaches ForkJoinPool's member class ForkJoinWorkerThreadFactory through its
// pickle before the std's ForkJoinPool enters the JDK's member classes: the class, settled as an empty trait, is read
// from its class file as it is filled, and the build goes on to what the JavaScript layer lacks.
object Main {
  def main(args: Array[String]): Unit = println(scala.concurrent.ExecutionContext.global != null)
}
