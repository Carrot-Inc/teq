// expect: std_method_erased_like_implementation.scala:7:7: error: class Task inherits call as (): String and as [T](): String
// A class that implements `java.util.concurrent.Callable[String]`'s `call` beside a trait's `call` with a type
// parameter of its own: the class's method and the trait's erase to the same result with no parameter, and scalac
// 3.8.4 reports E120, a name clash between a defined and an inherited member, at the class.
trait Counted { def call[T](): String = "other" }

class Task extends java.util.concurrent.Callable[String] with Counted { def call(): String = "std" }
