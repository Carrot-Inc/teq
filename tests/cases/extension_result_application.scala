class Operations {
  def add(text: String): String = text + "!"
  extension (base: Int) {
    def add: Int => Int = value => base + value
    def keep: [A] => A => A = [A] => (value: A) => value
    def viaFunction(value: Int): Int = add(value)
    def viaPoly[A](value: A): A = keep(value)
    def shadowed(value: Int): Int = {
      def add(x: Int): Int = x * 3
      add(value)
    }
    def selected(value: Int): Int = base.add(value)
    def objectShadow(value: Int): Int = {
      object add { def apply(x: Int): Int = x * 5 }
      add(value)
    }
    def classShadow(value: Int): Int = {
      class Inner {
        def add(x: Int): Int = x * 6
        def run: Int = add(value)
      }
      (new Inner).run
    }

  }
}
class PolyOperations {
  extension (base: Int) {
    def select: [A] => A => A = [A] => (value: A) => value
    def viaResult[A](value: A): A = select(value)
  }
}
object OuterOperations {
  def pick(value: String): String = value
  class InnerOperations {
    extension (base: Int) {
      def pick: Int => Int = value => base + value
      def throughSibling(value: Int): Int = pick(value)
    }
  }
}
@main def runExtensionResultApplication(): Unit = {
  val operations = new Operations
  import operations.*
  println(10.viaFunction(2))
  println(10.viaPoly("kept"))
  println(10.shadowed(2))
  println(10.selected(2))
  println(operations.add("text"))
  val poly = new PolyOperations
  import poly.*
  println(10.viaResult("class"))
  println(10.objectShadow(2))
  println(10.classShadow(2))
  val inner = new OuterOperations.InnerOperations
  import inner.*
  println(10.throughSibling(2))
}
