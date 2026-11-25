// teq: --std=scala-library
// expect: 9:26: error: illegal trait inheritance: superclass Foo does not derive from trait NoStackTrace's superclass Throwable
// expect: 1 error found
// A class whose superclass does not derive from the class a library trait among its parents extends (the trait read
// from scala-library's jar): scalac's "illegal trait inheritance", as a source trait's (inherit_parents).
import scala.util.control.NoStackTrace

class Foo
class X extends Foo with NoStackTrace
