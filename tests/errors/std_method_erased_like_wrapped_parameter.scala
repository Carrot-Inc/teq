// expect: std_method_erased_like_wrapped_parameter.scala:8:7: error: class Pool inherits execute as (command: Runnable): Unit and as (r: Runnable): Unit
// A class that implements `java.util.concurrent.Executor`'s `execute` beside a trait's `execute` whose parameter
// is a value class over `java.lang.Runnable`: the two erase alike, and scalac 3.8.4 refuses the class, the bridge
// of the one clashing with the other.
package wrapped { class Runnable(val r: java.lang.Runnable) extends AnyVal }

trait Jobs { def execute(r: wrapped.Runnable): Unit = () }
class Pool extends java.util.concurrent.Executor with Jobs { def execute(command: java.lang.Runnable): Unit = command.run() }
