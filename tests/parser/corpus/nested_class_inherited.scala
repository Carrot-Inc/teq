// A class nested in a class or trait is a member its subclasses name by its simple name, and the
// subclass's `this` is the enclosing instance of what they make.
class A(val n: Int):
  class C:
    def get = n

class B extends A(5):
  def make = new C

trait Named:
  def name: String
  class Tag:
    def show = s"<$name>"

object Item extends Named:
  def name = "item"
  def tag = new Tag

@main def run(): Unit =
  println(new B().make.get)
  println(Item.tag.show)
