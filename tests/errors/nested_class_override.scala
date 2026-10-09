// expect: 10:9: error: class Visitor cannot have the same name as trait Visitor in trait Base -- class definitions cannot be overridden
// A class nested in a class is inherited, and a subclass's class of the same name does not
// override it: scalac rejects it.
trait Base:
  trait Visitor:
    def visit(): Unit
  class Printer extends Visitor:
    def visit(): Unit = ()
trait Derived extends Base:
  class Visitor extends Printer
